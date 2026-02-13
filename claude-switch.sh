#!/usr/bin/env zsh
# ============================================================
# Claude Code Account Switcher (macOS / zsh)
# ============================================================
# Bind Claude Code accounts to specific directories.
#
# Install: add to ~/.zshrc:
#   source /path/to/claude-switch.sh
#
# Language: auto-detected from LANG, override with CLAUDE_ACC_LANG=en|ru
#
# Usage:
#   claude-acc                     — help
#   claude-acc list                — list accounts
#   claude-acc add <name>          — add account (opens login)
#   claude-acc remove <name>       — remove account
#   claude-acc default [name]      — show/set default account
#   claude-acc link <name>         — link account to current directory
#   claude-acc unlink              — unlink current directory
#   claude-acc status              — show active account
# ============================================================

CLAUDE_SWITCH_DIR="$HOME/.claude-switch"
CLAUDE_SWITCH_ACCOUNTS_DIR="$CLAUDE_SWITCH_DIR/accounts"
CLAUDE_SWITCH_CONFIG="$CLAUDE_SWITCH_DIR/config"
CLAUDE_SWITCH_LINKS="$CLAUDE_SWITCH_DIR/links"

# =============================================================
# Локализация / i18n
# =============================================================

_claude_acc_lang() {
    if [[ -n "$CLAUDE_ACC_LANG" ]]; then
        echo "$CLAUDE_ACC_LANG"
    elif [[ "$LANG" == ru_* ]]; then
        echo "ru"
    else
        echo "en"
    fi
}

typeset -gA _claude_msg_en _claude_msg_ru

_claude_msg_en=(
    help_title          "Claude Code Account Switcher"
    help_commands       "Commands:"
    help_list           "List accounts"
    help_add            "Add account"
    help_remove         "Remove account"
    help_default        "Show/set default account"
    help_reset          "Reset default to ~/.claude/"
    help_link           "Link account to current directory"
    help_unlink         "Unlink current directory"
    help_status         "Current account and context"
    help_help           "Help"
    list_empty          "No accounts. Add one: claude-acc add <name>"
    list_header         "Claude Code accounts:"
    list_default        "(default)"
    add_usage           "Usage: claude-acc add <name>"
    add_example         "Example: claude-acc add personal"
    add_exists          "Account '%s' already exists."
    add_created         "Account '%s' created. Starting login..."
    add_done            "Done. Use:"
    add_hint_default    "  claude-acc default %s   — set as default"
    add_hint_link       "  claude-acc link %s      — link to current directory"
    remove_usage        "Usage: claude-acc remove <name>"
    remove_not_found    "Account '%s' not found."
    remove_deleted      "Account '%s' deleted."
    default_current     "Default: %s"
    default_standard    "Default: ~/.claude/"
    default_not_found   "Account '%s' not found. Available:"
    default_set         "Default account: %s"
    reset_done          "Reset to ~/.claude/"
    link_usage          "Usage: claude-acc link <name>"
    link_desc           "Links account to the current directory."
    link_not_found      "Account '%s' not found. Available:"
    link_done           "%s → account '%s'"
    unlink_none         "No link for the current directory."
    unlink_done         "Unlinked %s. Default account will be used."
    status_active       "Active account: %s %s"
    status_linked       "(linked to %s)"
    status_default      "(default)"
    status_standard     "Active account: ~/.claude/ (standard)"
)

_claude_msg_ru=(
    help_title          "Claude Code Account Switcher"
    help_commands       "Команды:"
    help_list           "Список аккаунтов"
    help_add            "Добавить аккаунт"
    help_remove         "Удалить аккаунт"
    help_default        "Показать/задать дефолтный аккаунт"
    help_reset          "Сбросить дефолт на ~/.claude/"
    help_link           "Привязать аккаунт к текущей директории"
    help_unlink         "Убрать привязку с текущей директории"
    help_status         "Текущий аккаунт и контекст"
    help_help           "Справка"
    list_empty          "Нет аккаунтов. Добавьте: claude-acc add <name>"
    list_header         "Аккаунты Claude Code:"
    list_default        "(по умолчанию)"
    add_usage           "Использование: claude-acc add <name>"
    add_example         "Пример:        claude-acc add personal"
    add_exists          "Аккаунт '%s' уже существует."
    add_created         "Аккаунт '%s' создан. Запускаю логин..."
    add_done            "Готово. Используйте:"
    add_hint_default    "  claude-acc default %s   — сделать дефолтным"
    add_hint_link       "  claude-acc link %s      — привязать к текущей директории"
    remove_usage        "Использование: claude-acc remove <name>"
    remove_not_found    "Аккаунт '%s' не найден."
    remove_deleted      "Аккаунт '%s' удалён."
    default_current     "По умолчанию: %s"
    default_standard    "По умолчанию: ~/.claude/"
    default_not_found   "Аккаунт '%s' не найден. Доступные:"
    default_set         "Аккаунт по умолчанию: %s"
    reset_done          "Сброшено на ~/.claude/"
    link_usage          "Использование: claude-acc link <name>"
    link_desc           "Привязывает аккаунт к текущей директории."
    link_not_found      "Аккаунт '%s' не найден. Доступные:"
    link_done           "%s → аккаунт '%s'"
    unlink_none         "Нет привязки для текущей директории."
    unlink_done         "Привязка убрана для %s. Будет использован дефолтный аккаунт."
    status_active       "Активный аккаунт: %s %s"
    status_linked       "(привязан к %s)"
    status_default      "(по умолчанию)"
    status_standard     "Активный аккаунт: ~/.claude/ (стандартный)"
)

_msg() {
    local key="$1"
    shift
    local lang=$(_claude_acc_lang)
    local template

    if [[ "$lang" == "ru" ]]; then
        template="${_claude_msg_ru[$key]}"
    else
        template="${_claude_msg_en[$key]}"
    fi

    if [[ $# -gt 0 ]]; then
        printf "$template\n" "$@"
    else
        echo "$template"
    fi
}

# =============================================================
# Ядро
# =============================================================

# --- Инициализация ---
_claude_switch_init() {
    mkdir -p "$CLAUDE_SWITCH_ACCOUNTS_DIR"
    [[ -f "$CLAUDE_SWITCH_CONFIG" ]] || echo "default=" > "$CLAUDE_SWITCH_CONFIG"
    [[ -f "$CLAUDE_SWITCH_LINKS" ]]  || touch "$CLAUDE_SWITCH_LINKS"
    # Миграция: переименовать repos → links
    if [[ -f "$CLAUDE_SWITCH_DIR/repos" && ! -s "$CLAUDE_SWITCH_LINKS" ]]; then
        mv "$CLAUDE_SWITCH_DIR/repos" "$CLAUDE_SWITCH_LINKS"
    fi
}
_claude_switch_init

# --- Прочитать дефолтный аккаунт ---
_claude_default_account() {
    grep '^default=' "$CLAUDE_SWITCH_CONFIG" 2>/dev/null | cut -d= -f2
}

# --- Найти аккаунт для директории (точное совпадение) ---
_claude_dir_account() {
    local dir="$1"
    [[ -z "$dir" ]] && return 1
    grep -F "${dir}=" "$CLAUDE_SWITCH_LINKS" 2>/dev/null | grep "^${dir}=" | head -1 | cut -d= -f2
}

# --- Найти аккаунт, поднимаясь по дереву директорий ---
_claude_find_account() {
    local dir="${1:-$PWD}"
    local account

    while [[ "$dir" != "/" && -n "$dir" ]]; do
        account=$(_claude_dir_account "$dir")
        if [[ -n "$account" ]]; then
            echo "$account"
            return 0
        fi
        dir="${dir:h}"
    done

    return 1
}

# --- Найти директорию привязки (для status) ---
_claude_find_linked_dir() {
    local dir="${1:-$PWD}"

    while [[ "$dir" != "/" && -n "$dir" ]]; do
        if grep -qF "${dir}=" "$CLAUDE_SWITCH_LINKS" 2>/dev/null && \
           grep -q "^${dir}=" "$CLAUDE_SWITCH_LINKS" 2>/dev/null; then
            echo "$dir"
            return 0
        fi
        dir="${dir:h}"
    done

    return 1
}

# --- Установить CLAUDE_CONFIG_DIR для текущего контекста ---
_claude_activate() {
    local account
    account=$(_claude_find_account)

    if [[ -z "$account" ]]; then
        account=$(_claude_default_account)
    fi

    if [[ -n "$account" && -d "$CLAUDE_SWITCH_ACCOUNTS_DIR/$account" ]]; then
        export CLAUDE_CONFIG_DIR="$CLAUDE_SWITCH_ACCOUNTS_DIR/$account"
    else
        unset CLAUDE_CONFIG_DIR
    fi
}

# --- Хук на cd: автоматически переключает аккаунт ---
_claude_chpwd_hook() {
    _claude_activate
}

# Регистрируем хук (zsh вызывает chpwd при каждой смене директории)
autoload -Uz add-zsh-hook
add-zsh-hook chpwd _claude_chpwd_hook

# Активировать сразу для текущей директории
_claude_activate

# =============================================================
# Подкоманды
# =============================================================

_claude_acc_help() {
    _msg help_title
    echo ""
    _msg help_commands
    echo "  claude-acc list              $(_msg help_list)"
    echo "  claude-acc add <name>        $(_msg help_add)"
    echo "  claude-acc remove <name>     $(_msg help_remove)"
    echo "  claude-acc default [name]    $(_msg help_default)"
    echo "  claude-acc reset             $(_msg help_reset)"
    echo "  claude-acc link <name>       $(_msg help_link)"
    echo "  claude-acc unlink            $(_msg help_unlink)"
    echo "  claude-acc status            $(_msg help_status)"
}

_claude_acc_list() {
    local default_acc
    default_acc=$(_claude_default_account)

    local accounts=("$CLAUDE_SWITCH_ACCOUNTS_DIR"/*(N:t))
    if [[ ${#accounts} -eq 0 ]]; then
        _msg list_empty
        return
    fi

    _msg list_header
    for acc in "${accounts[@]}"; do
        if [[ "$acc" == "$default_acc" ]]; then
            echo "  ★ $acc  $(_msg list_default)"
        else
            echo "    $acc"
        fi
    done
}

_claude_acc_add() {
    local name="$1"
    if [[ -z "$name" ]]; then
        _msg add_usage
        _msg add_example
        return 1
    fi

    local acc_dir="$CLAUDE_SWITCH_ACCOUNTS_DIR/$name"
    if [[ -d "$acc_dir" ]]; then
        _msg add_exists "$name"
        return 1
    fi

    mkdir -p "$acc_dir"
    _msg add_created "$name"
    CLAUDE_CONFIG_DIR="$acc_dir" claude login
    echo ""
    _msg add_done
    _msg add_hint_default "$name"
    _msg add_hint_link "$name"
}

_claude_acc_remove() {
    local name="$1"
    if [[ -z "$name" ]]; then
        _msg remove_usage
        return 1
    fi

    local acc_dir="$CLAUDE_SWITCH_ACCOUNTS_DIR/$name"
    if [[ ! -d "$acc_dir" ]]; then
        _msg remove_not_found "$name"
        return 1
    fi

    # Убрать из дефолтного
    local default_acc
    default_acc=$(_claude_default_account)
    if [[ "$default_acc" == "$name" ]]; then
        sed -i '' "s/^default=.*/default=/" "$CLAUDE_SWITCH_CONFIG"
    fi

    # Убрать привязки
    sed -i '' "/=$name$/d" "$CLAUDE_SWITCH_LINKS"

    rm -rf "$acc_dir"
    _msg remove_deleted "$name"
    _claude_activate
}

_claude_acc_default() {
    local name="$1"
    if [[ -z "$name" ]]; then
        local current
        current=$(_claude_default_account)
        if [[ -n "$current" ]]; then
            _msg default_current "$current"
        else
            _msg default_standard
        fi
        return
    fi

    if [[ ! -d "$CLAUDE_SWITCH_ACCOUNTS_DIR/$name" ]]; then
        _msg default_not_found "$name"
        _claude_acc_list
        return 1
    fi

    sed -i '' "s/^default=.*/default=$name/" "$CLAUDE_SWITCH_CONFIG"
    _msg default_set "$name"
    _claude_activate
}

_claude_acc_link() {
    local name="$1"
    if [[ -z "$name" ]]; then
        _msg link_usage
        _msg link_desc
        return 1
    fi

    if [[ ! -d "$CLAUDE_SWITCH_ACCOUNTS_DIR/$name" ]]; then
        _msg link_not_found "$name"
        _claude_acc_list
        return 1
    fi

    local dir="$PWD"

    # Убрать старую привязку для этой директории, если есть
    sed -i '' "\|^${dir}=|d" "$CLAUDE_SWITCH_LINKS"

    # Добавить новую
    echo "${dir}=${name}" >> "$CLAUDE_SWITCH_LINKS"
    _msg link_done "$(basename "$dir")" "$name"
    _claude_activate
}

_claude_acc_unlink() {
    local dir="$PWD"

    if ! grep -qF "${dir}=" "$CLAUDE_SWITCH_LINKS" 2>/dev/null || \
       ! grep -q "^${dir}=" "$CLAUDE_SWITCH_LINKS" 2>/dev/null; then
        _msg unlink_none
        return 1
    fi

    sed -i '' "\|^${dir}=|d" "$CLAUDE_SWITCH_LINKS"
    _msg unlink_done "$(basename "$dir")"
    _claude_activate
}

_claude_acc_status() {
    local account source_info linked_dir

    linked_dir=$(_claude_find_linked_dir)
    if [[ -n "$linked_dir" ]]; then
        account=$(_claude_dir_account "$linked_dir")
        source_info=$(_msg status_linked "$(basename "$linked_dir")")
    fi

    if [[ -z "$account" ]]; then
        account=$(_claude_default_account)
        if [[ -n "$account" ]]; then
            source_info=$(_msg status_default)
        fi
    fi

    if [[ -n "$account" ]]; then
        _msg status_active "$account" "$source_info"
    else
        _msg status_standard
    fi
}

_claude_acc_reset() {
    sed -i '' "s/^default=.*/default=/" "$CLAUDE_SWITCH_CONFIG"
    _msg reset_done
    _claude_activate
}

# =============================================================
# Единая точка входа
# =============================================================

claude-acc() {
    local cmd="$1"
    shift 2>/dev/null

    case "$cmd" in
        list)    _claude_acc_list "$@" ;;
        add)     _claude_acc_add "$@" ;;
        remove)  _claude_acc_remove "$@" ;;
        default) _claude_acc_default "$@" ;;
        reset)   _claude_acc_reset ;;
        link)    _claude_acc_link "$@" ;;
        unlink)  _claude_acc_unlink "$@" ;;
        status)  _claude_acc_status "$@" ;;
        help)    _claude_acc_help ;;
        *)       _claude_acc_help ;;
    esac
}

# =============================================================
# Автодополнение (zsh)
# =============================================================

_claude_acc_completion() {
    local -a subcmds accounts
    subcmds=(
        "list:$(_msg help_list)"
        "add:$(_msg help_add)"
        "remove:$(_msg help_remove)"
        "default:$(_msg help_default)"
        "reset:$(_msg help_reset)"
        "link:$(_msg help_link)"
        "unlink:$(_msg help_unlink)"
        "status:$(_msg help_status)"
        "help:$(_msg help_help)"
    )

    if (( CURRENT == 2 )); then
        _describe 'command' subcmds
    elif (( CURRENT == 3 )); then
        case "${words[2]}" in
            remove|default|link)
                accounts=("$CLAUDE_SWITCH_ACCOUNTS_DIR"/*(N:t))
                _describe 'account' accounts
                ;;
        esac
    fi
}

compdef _claude_acc_completion claude-acc
