#!/usr/bin/env zsh
# ============================================================
# Claude Code Account Switcher (macOS / zsh)
# ============================================================
# Привязка аккаунтов Claude Code к конкретным репозиториям.
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
#   claude-acc link <name>         — привязать аккаунт к текущему репо
#   claude-acc unlink              — убрать привязку с текущего репо
#   claude-acc status              — показать активный аккаунт
# ============================================================

CLAUDE_SWITCH_DIR="$HOME/.claude-switch"
CLAUDE_SWITCH_ACCOUNTS_DIR="$CLAUDE_SWITCH_DIR/accounts"
CLAUDE_SWITCH_CONFIG="$CLAUDE_SWITCH_DIR/config"
CLAUDE_SWITCH_REPOS="$CLAUDE_SWITCH_DIR/repos"

# --- Инициализация ---
_claude_switch_init() {
    mkdir -p "$CLAUDE_SWITCH_ACCOUNTS_DIR"
    [[ -f "$CLAUDE_SWITCH_CONFIG" ]] || echo "default=" > "$CLAUDE_SWITCH_CONFIG"
    [[ -f "$CLAUDE_SWITCH_REPOS" ]]   || touch "$CLAUDE_SWITCH_REPOS"
}
_claude_switch_init

# --- Получить корень git-репо (или пустую строку) ---
_claude_git_root() {
    git -C "${1:-.}" rev-parse --show-toplevel 2>/dev/null
}

# --- Прочитать дефолтный аккаунт ---
_claude_default_account() {
    grep '^default=' "$CLAUDE_SWITCH_CONFIG" 2>/dev/null | cut -d= -f2
}

# --- Найти аккаунт для репо ---
_claude_repo_account() {
    local repo_root="$1"
    [[ -z "$repo_root" ]] && return 1
    grep -F "${repo_root}=" "$CLAUDE_SWITCH_REPOS" 2>/dev/null | grep "^${repo_root}=" | cut -d= -f2
}

# --- Установить CLAUDE_CONFIG_DIR для текущего контекста ---
_claude_activate() {
    local repo_root account
    repo_root=$(_claude_git_root)

    if [[ -n "$repo_root" ]]; then
        account=$(_claude_repo_account "$repo_root")
    fi

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
    echo "  claude-acc link <name>       Привязать аккаунт к текущему репо"
    echo "  claude-acc unlink            Убрать привязку с текущего репо"
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
    echo "  claude-acc link $name      — привязать к текущему репо"
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

    # Убрать привязки репо
    sed -i '' "/=$name$/d" "$CLAUDE_SWITCH_REPOS"

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
            echo "Дефолтный аккаунт не задан."
            echo "Использование: claude-acc default <name>"
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
        echo "Привязывает аккаунт к текущему git-репозиторию."
        return 1
    fi

    if [[ ! -d "$CLAUDE_SWITCH_ACCOUNTS_DIR/$name" ]]; then
        echo "Аккаунт '$name' не найден. Доступные:"
        _claude_acc_list
        return 1
    fi

    local repo_root
    repo_root=$(_claude_git_root)
    if [[ -z "$repo_root" ]]; then
        echo "Ошибка: текущая директория не в git-репозитории."
        return 1
    fi

    # Убрать старую привязку для этого репо, если есть
    sed -i '' "\|^${repo_root}=|d" "$CLAUDE_SWITCH_REPOS"

    # Добавить новую
    echo "${repo_root}=${name}" >> "$CLAUDE_SWITCH_REPOS"
    echo "Репо $(basename "$repo_root") → аккаунт '$name'"
    _claude_activate
}

_claude_acc_unlink() {
    local repo_root
    repo_root=$(_claude_git_root)
    if [[ -z "$repo_root" ]]; then
        echo "Ошибка: текущая директория не в git-репозитории."
        return 1
    fi

    sed -i '' "\|^${repo_root}=|d" "$CLAUDE_SWITCH_REPOS"
    echo "Привязка убрана для $(basename "$repo_root"). Будет использован дефолтный аккаунт."
    _claude_activate
}

_claude_acc_status() {
    local repo_root account source_info

    repo_root=$(_claude_git_root)
    if [[ -n "$repo_root" ]]; then
        account=$(_claude_repo_account "$repo_root")
        if [[ -n "$account" ]]; then
            source_info="(привязан к $(basename "$repo_root"))"
        fi
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
        echo "Аккаунт не выбран. Настройте:"
        echo "  claude-acc add <name>      — добавить аккаунт"
        echo "  claude-acc default <name>  — задать дефолтный"
    fi

    if [[ -n "$repo_root" ]]; then
        local bound
        bound=$(_claude_repo_account "$repo_root")
        if [[ -z "$bound" ]]; then
            echo "Репо $(basename "$repo_root"): нет привязки (используется дефолт)"
        fi
    fi
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
        'link:Привязать аккаунт к текущему репо'
        'unlink:Убрать привязку с текущего репо'
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
