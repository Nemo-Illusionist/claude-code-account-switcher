#!/usr/bin/env zsh
# ============================================================
# Claude Code Account Switcher (macOS / zsh)
# ============================================================
# Привязка аккаунтов Claude Code к конкретным директориям.
#
# Установка: добавьте в ~/.zshrc:
#   source /path/to/claude-switch.sh
#
# Использование:
#   claude-acc                     — справка
#   claude-acc list                — список аккаунтов
#   claude-acc add <name>          — добавить аккаунт (откроет логин)
#   claude-acc remove <name>       — удалить аккаунт
#   claude-acc default [name]      — показать/задать аккаунт по умолчанию
#   claude-acc link <name>         — привязать аккаунт к текущей директории
#   claude-acc unlink              — убрать привязку с текущей директории
#   claude-acc status              — показать активный аккаунт
# ============================================================

CLAUDE_SWITCH_DIR="$HOME/.claude-switch"
CLAUDE_SWITCH_ACCOUNTS_DIR="$CLAUDE_SWITCH_DIR/accounts"
CLAUDE_SWITCH_CONFIG="$CLAUDE_SWITCH_DIR/config"
CLAUDE_SWITCH_LINKS="$CLAUDE_SWITCH_DIR/links"

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
    echo "Claude Code Account Switcher"
    echo ""
    echo "Команды:"
    echo "  claude-acc list              Список аккаунтов"
    echo "  claude-acc add <name>        Добавить аккаунт"
    echo "  claude-acc remove <name>     Удалить аккаунт"
    echo "  claude-acc default [name]    Показать/задать дефолтный аккаунт"
    echo "  claude-acc reset             Сбросить дефолт на ~/.claude/"
    echo "  claude-acc link <name>       Привязать аккаунт к текущей директории"
    echo "  claude-acc unlink            Убрать привязку с текущей директории"
    echo "  claude-acc status            Текущий аккаунт и контекст"
}

_claude_acc_list() {
    local default_acc
    default_acc=$(_claude_default_account)

    local accounts=("$CLAUDE_SWITCH_ACCOUNTS_DIR"/*(N:t))
    if [[ ${#accounts} -eq 0 ]]; then
        echo "Нет аккаунтов. Добавьте: claude-acc add <name>"
        return
    fi

    echo "Аккаунты Claude Code:"
    for acc in "${accounts[@]}"; do
        if [[ "$acc" == "$default_acc" ]]; then
            echo "  ★ $acc  (по умолчанию)"
        else
            echo "    $acc"
        fi
    done
}

_claude_acc_add() {
    local name="$1"
    if [[ -z "$name" ]]; then
        echo "Использование: claude-acc add <name>"
        echo "Пример:        claude-acc add personal"
        return 1
    fi

    local acc_dir="$CLAUDE_SWITCH_ACCOUNTS_DIR/$name"
    if [[ -d "$acc_dir" ]]; then
        echo "Аккаунт '$name' уже существует."
        return 1
    fi

    mkdir -p "$acc_dir"
    echo "Аккаунт '$name' создан. Запускаю логин..."
    CLAUDE_CONFIG_DIR="$acc_dir" claude login
    echo ""
    echo "Готово. Используйте:"
    echo "  claude-acc default $name   — сделать дефолтным"
    echo "  claude-acc link $name      — привязать к текущей директории"
}

_claude_acc_remove() {
    local name="$1"
    if [[ -z "$name" ]]; then
        echo "Использование: claude-acc remove <name>"
        return 1
    fi

    local acc_dir="$CLAUDE_SWITCH_ACCOUNTS_DIR/$name"
    if [[ ! -d "$acc_dir" ]]; then
        echo "Аккаунт '$name' не найден."
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
    echo "Аккаунт '$name' удалён."
    _claude_activate
}

_claude_acc_default() {
    local name="$1"
    if [[ -z "$name" ]]; then
        local current
        current=$(_claude_default_account)
        if [[ -n "$current" ]]; then
            echo "По умолчанию: $current"
        else
            echo "По умолчанию: ~/.claude/"
        fi
        return
    fi

    if [[ ! -d "$CLAUDE_SWITCH_ACCOUNTS_DIR/$name" ]]; then
        echo "Аккаунт '$name' не найден. Доступные:"
        _claude_acc_list
        return 1
    fi

    sed -i '' "s/^default=.*/default=$name/" "$CLAUDE_SWITCH_CONFIG"
    echo "Аккаунт по умолчанию: $name"
    _claude_activate
}

_claude_acc_link() {
    local name="$1"
    if [[ -z "$name" ]]; then
        echo "Использование: claude-acc link <name>"
        echo "Привязывает аккаунт к текущей директории."
        return 1
    fi

    if [[ ! -d "$CLAUDE_SWITCH_ACCOUNTS_DIR/$name" ]]; then
        echo "Аккаунт '$name' не найден. Доступные:"
        _claude_acc_list
        return 1
    fi

    local dir="$PWD"

    # Убрать старую привязку для этой директории, если есть
    sed -i '' "\|^${dir}=|d" "$CLAUDE_SWITCH_LINKS"

    # Добавить новую
    echo "${dir}=${name}" >> "$CLAUDE_SWITCH_LINKS"
    echo "$(basename "$dir") → аккаунт '$name'"
    _claude_activate
}

_claude_acc_unlink() {
    local dir="$PWD"

    if ! grep -qF "${dir}=" "$CLAUDE_SWITCH_LINKS" 2>/dev/null || \
       ! grep -q "^${dir}=" "$CLAUDE_SWITCH_LINKS" 2>/dev/null; then
        echo "Нет привязки для текущей директории."
        return 1
    fi

    sed -i '' "\|^${dir}=|d" "$CLAUDE_SWITCH_LINKS"
    echo "Привязка убрана для $(basename "$dir"). Будет использован дефолтный аккаунт."
    _claude_activate
}

_claude_acc_status() {
    local account source_info linked_dir

    linked_dir=$(_claude_find_linked_dir)
    if [[ -n "$linked_dir" ]]; then
        account=$(_claude_dir_account "$linked_dir")
        source_info="(привязан к $(basename "$linked_dir"))"
    fi

    if [[ -z "$account" ]]; then
        account=$(_claude_default_account)
        if [[ -n "$account" ]]; then
            source_info="(по умолчанию)"
        fi
    fi

    if [[ -n "$account" ]]; then
        echo "Активный аккаунт: $account $source_info"
    else
        echo "Активный аккаунт: ~/.claude/ (стандартный)"
    fi
}

_claude_acc_reset() {
    sed -i '' "s/^default=.*/default=/" "$CLAUDE_SWITCH_CONFIG"
    echo "Сброшено на ~/.claude/"
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
        'list:Список аккаунтов'
        'add:Добавить аккаунт'
        'remove:Удалить аккаунт'
        'default:Показать/задать дефолтный аккаунт'
        'reset:Сбросить дефолт на ~/.claude/'
        'link:Привязать аккаунт к текущей директории'
        'unlink:Убрать привязку с текущей директории'
        'status:Текущий аккаунт и контекст'
        'help:Справка'
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
