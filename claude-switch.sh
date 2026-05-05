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
CLAUDE_SWITCH_BIN="$CLAUDE_SWITCH_DIR/bin"
CLAUDE_SWITCH_SCRIPT="${(%):-%x}"
CLAUDE_SWITCH_SCRIPT="${CLAUDE_SWITCH_SCRIPT:A}"

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
    help_login          "Re-login to an account"
    help_remove         "Remove account"
    help_default        "Show/set default account"
    help_reset          "Reset default to ~/.claude/"
    help_link           "Link account to current directory"
    help_unlink         "Unlink current directory"
    help_links          "Show all directory links"
    help_status         "Current account and context"
    help_run            "Run claude under a specific account"
    help_doctor         "Audit each account OAuth identity (email, UUID)"
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
    login_usage         "Usage: claude-acc login <name>"
    login_not_found     "Account '%s' not found."
    login_start         "Logging in to '%s'..."
    login_done          "Done."
    remove_usage        "Usage: claude-acc remove <name>"
    remove_not_found    "Account '%s' not found."
    remove_confirm      "Remove account '%s'? [y/N] "
    remove_cancelled    "Cancelled."
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
    link_done_default   "%s → ~/.claude/ (default)"
    reserved_name       "'%s' is a reserved name."
    name_invalid        "Account name must contain only letters, digits, hyphens, and underscores."
    run_usage           "Usage: claude-acc run <name> [args...]"
    run_not_found       "Account '%s' not found."
    doctor_header       "Auditing %d account(s):"
    doctor_no_token     "no token (run: claude-acc login %s)"
    doctor_offline      "token present, but API unreachable"
    doctor_all_ok       "All accounts healthy."
    doctor_partial      "%d of %d accounts healthy."
    doctor_missing_dep  "claude-acc doctor needs '%s' on PATH."
    unlink_none         "No link for the current directory."
    unlink_done         "Unlinked %s. Default account will be used."
    status_active       "Active account: %s %s"
    status_linked       "(linked to %s)"
    status_default      "(default)"
    status_standard     "Active account: ~/.claude/ (standard)"
    links_empty         "No links. Use: claude-acc link <name>"
    links_header        "Links:"
    links_active        "← active"
)

_claude_msg_ru=(
    help_title          "Claude Code Account Switcher"
    help_commands       "Команды:"
    help_list           "Список аккаунтов"
    help_add            "Добавить аккаунт"
    help_login          "Перелогиниться в аккаунт"
    help_remove         "Удалить аккаунт"
    help_default        "Показать/задать дефолтный аккаунт"
    help_reset          "Сбросить дефолт на ~/.claude/"
    help_link           "Привязать аккаунт к текущей директории"
    help_unlink         "Убрать привязку с текущей директории"
    help_links          "Показать все привязки директорий"
    help_status         "Текущий аккаунт и контекст"
    help_run            "Запустить claude под конкретным аккаунтом"
    help_doctor         "Аудит OAuth-личности каждого аккаунта (email, UUID)"
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
    login_usage         "Использование: claude-acc login <name>"
    login_not_found     "Аккаунт '%s' не найден."
    login_start         "Вхожу в '%s'..."
    login_done          "Готово."
    remove_usage        "Использование: claude-acc remove <name>"
    remove_not_found    "Аккаунт '%s' не найден."
    remove_confirm      "Удалить аккаунт '%s'? [y/N] "
    remove_cancelled    "Отменено."
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
    link_done_default   "%s → ~/.claude/ (default)"
    reserved_name       "'%s' — зарезервированное имя."
    name_invalid        "Имя аккаунта может содержать только буквы, цифры, дефисы и подчёркивания."
    run_usage           "Использование: claude-acc run <name> [args...]"
    run_not_found       "Аккаунт '%s' не найден."
    doctor_header       "Проверка %d аккаунт(ов):"
    doctor_no_token     "нет токена (запустите: claude-acc login %s)"
    doctor_offline      "токен есть, API недоступен"
    doctor_all_ok       "Все аккаунты в порядке."
    doctor_partial      "%d из %d аккаунтов в порядке."
    doctor_missing_dep  "claude-acc doctor требует '%s' в PATH."
    unlink_none         "Нет привязки для текущей директории."
    unlink_done         "Привязка убрана для %s. Будет использован дефолтный аккаунт."
    status_active       "Активный аккаунт: %s %s"
    status_linked       "(привязан к %s)"
    status_default      "(по умолчанию)"
    status_standard     "Активный аккаунт: ~/.claude/ (стандартный)"
    links_empty         "Нет привязок. Используйте: claude-acc link <name>"
    links_header        "Привязки:"
    links_active        "← активна"
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

# --- Валидация имени аккаунта ---
# Allowed: ASCII letters, digits, hyphens, underscores. Rejects path
# separators, regex metacharacters, whitespace, and unicode — anything
# that could be unsafe inside file paths or grep/sed patterns.
_claude_validate_name() {
    local name="$1"
    if [[ ! "$name" =~ ^[a-zA-Z0-9_-]+$ ]]; then
        _msg name_invalid
        return 1
    fi
}

# --- Инициализация ---
_claude_switch_init() {
    mkdir -p "$CLAUDE_SWITCH_ACCOUNTS_DIR"
    mkdir -p "$CLAUDE_SWITCH_BIN"
    [[ -f "$CLAUDE_SWITCH_CONFIG" ]] || echo "default=" > "$CLAUDE_SWITCH_CONFIG"
    [[ -f "$CLAUDE_SWITCH_LINKS" ]]  || touch "$CLAUDE_SWITCH_LINKS"
    # Миграция: переименовать repos → links
    if [[ -f "$CLAUDE_SWITCH_DIR/repos" && ! -s "$CLAUDE_SWITCH_LINKS" ]]; then
        mv "$CLAUDE_SWITCH_DIR/repos" "$CLAUDE_SWITCH_LINKS"
    fi
    # Обеспечить ide/ symlinks и wrapper для IDE-интеграции
    _claude_ensure_ide_symlinks
    _claude_ensure_wrapper
}

# --- Создать ide/ symlinks для всех аккаунтов ---
# Все аккаунты указывают на ~/.claude/ide/ чтобы IDE-плагины (PhpStorm, VSCode)
# всегда находили lock-файлы Claude Code в стандартном месте.
_claude_ensure_ide_symlinks() {
    local ide_dir="$HOME/.claude/ide"
    mkdir -p "$ide_dir"
    for acc_dir in "$CLAUDE_SWITCH_ACCOUNTS_DIR"/*(N/); do
        local acc_ide="$acc_dir/ide"
        if [[ ! -L "$acc_ide" ]]; then
            rm -rf "$acc_ide"
            ln -s "$ide_dir" "$acc_ide"
        fi
    done
}

# --- Создать wrapper-скрипт ~/.claude-switch/bin/claude ---
# Нужен чтобы IDE (PhpStorm и др.) запускали claude с правильным CLAUDE_CONFIG_DIR.
# IDE запускает бинарник напрямую без .zshrc, поэтому wrapper сам source-ит
# claude-switch.sh — это поднимает _claude_activate против $PWD и переиспользует
# ту же логику lookup'а что и терминальный chpwd-хук (без дублирования кода).
_claude_ensure_wrapper() {
    local wrapper="$CLAUDE_SWITCH_BIN/claude"

    # Перегенерировать только если wrapper отсутствует или старее главного скрипта
    if [[ -x "$wrapper" && "$wrapper" -nt "$CLAUDE_SWITCH_SCRIPT" ]]; then
        return
    fi

    cat > "$wrapper" <<WRAPPER_EOF
#!/usr/bin/env zsh
# Auto-generated by claude-switch — do not edit manually.
emulate -L zsh

[[ -r '$CLAUDE_SWITCH_SCRIPT' ]] && source '$CLAUDE_SWITCH_SCRIPT' >/dev/null 2>&1

_cs_self="\${0:A}"
_cs_self_dir="\${_cs_self:h}"
for _cs_dir in \${(s/:/)PATH}; do
    [[ "\$_cs_dir" == "\$_cs_self_dir" ]] && continue
    _cs_target="\$_cs_dir/claude"
    [[ -x "\$_cs_target" ]] || continue
    [[ "\${_cs_target:A}" == "\$_cs_self" ]] && continue
    exec "\$_cs_target" "\$@"
done
print -u2 "claude-switch: real 'claude' binary not found in PATH"
exit 127
WRAPPER_EOF
    chmod +x "$wrapper"
}
_claude_switch_init

# --- Безопасные операции с файлом links ---
# Используем pure-shell обработку вместо grep/sed по двум причинам:
# 1. Пути могут содержать regex-метасимволы (`.`, `[`, `+` и т.п. — частая
#    история на macOS), которые ломают anchored-grep over-match'ем.
# 2. `sed -i ''` для удаления строки тоже трактует pattern как regex.
# `[[ "$line" == "${dir}="* ]]` использует glob-pattern с буквальной частью
# и единственным `*` в конце — никаких сюрпризов с метасимволами в `$dir`.

_claude_links_has_dir() {
    local dir="$1"
    [[ -z "$dir" || ! -f "$CLAUDE_SWITCH_LINKS" ]] && return 1
    local line
    while IFS= read -r line; do
        [[ "$line" == "${dir}="* ]] && return 0
    done < "$CLAUDE_SWITCH_LINKS"
    return 1
}

_claude_links_remove_dir() {
    local dir="$1"
    [[ -z "$dir" || ! -f "$CLAUDE_SWITCH_LINKS" ]] && return 0
    local tmpfile line
    tmpfile=$(mktemp) || return 1
    while IFS= read -r line; do
        [[ "$line" == "${dir}="* ]] || printf '%s\n' "$line"
    done < "$CLAUDE_SWITCH_LINKS" > "$tmpfile"
    mv "$tmpfile" "$CLAUDE_SWITCH_LINKS"
}

# --- Прочитать дефолтный аккаунт ---
_claude_default_account() {
    grep '^default=' "$CLAUDE_SWITCH_CONFIG" 2>/dev/null | cut -d= -f2
}

# --- Найти аккаунт для директории (точное совпадение) ---
_claude_dir_account() {
    local dir="$1"
    [[ -z "$dir" || ! -f "$CLAUDE_SWITCH_LINKS" ]] && return 1
    local line
    while IFS= read -r line; do
        if [[ "$line" == "${dir}="* ]]; then
            echo "${line#*=}"
            return 0
        fi
    done < "$CLAUDE_SWITCH_LINKS"
    return 1
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
        if _claude_links_has_dir "$dir"; then
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

    if [[ "$account" == "default" ]]; then
        unset CLAUDE_CONFIG_DIR
    elif [[ -n "$account" && -d "$CLAUDE_SWITCH_ACCOUNTS_DIR/$account" ]]; then
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
    echo "  claude-acc login <name>      $(_msg help_login)"
    echo "  claude-acc remove <name>     $(_msg help_remove)"
    echo "  claude-acc default [name]    $(_msg help_default)"
    echo "  claude-acc reset             $(_msg help_reset)"
    echo "  claude-acc link <name>       $(_msg help_link)"
    echo "  claude-acc unlink            $(_msg help_unlink)"
    echo "  claude-acc links             $(_msg help_links)"
    echo "  claude-acc status            $(_msg help_status)"
    echo "  claude-acc run <name> [...]  $(_msg help_run)"
    echo "  claude-acc doctor            $(_msg help_doctor)"
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
    local suffix
    for acc in "${accounts[@]}"; do
        suffix=$(_claude_acc_identity_suffix "$acc")
        if [[ "$acc" == "$default_acc" ]]; then
            echo "  ★ $acc  $(_msg list_default)$suffix"
        else
            echo "    $acc$suffix"
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

    if [[ "$name" == "default" ]]; then
        _msg reserved_name "$name"
        return 1
    fi

    _claude_validate_name "$name" || return 1

    local acc_dir="$CLAUDE_SWITCH_ACCOUNTS_DIR/$name"
    if [[ -d "$acc_dir" ]]; then
        _msg add_exists "$name"
        return 1
    fi

    mkdir -p "$acc_dir"
    mkdir -p "$HOME/.claude/ide"
    ln -sfn "$HOME/.claude/ide" "$acc_dir/ide"
    _msg add_created "$name"
    CLAUDE_CONFIG_DIR="$acc_dir" claude auth login
    echo ""
    _msg add_done
    _msg add_hint_default "$name"
    _msg add_hint_link "$name"
}

_claude_acc_login() {
    local name="$1"
    if [[ -z "$name" ]]; then
        _msg login_usage
        return 1
    fi

    _claude_validate_name "$name" || return 1

    local acc_dir="$CLAUDE_SWITCH_ACCOUNTS_DIR/$name"
    if [[ ! -d "$acc_dir" ]]; then
        _msg login_not_found "$name"
        return 1
    fi

    _msg login_start "$name"
    CLAUDE_CONFIG_DIR="$acc_dir" claude auth login
    _msg login_done
}

_claude_acc_remove() {
    local force=false
    if [[ "$1" == "-f" ]]; then
        force=true
        shift
    fi

    local name="$1"
    if [[ -z "$name" ]]; then
        _msg remove_usage
        return 1
    fi

    if [[ "$name" == "default" ]]; then
        _msg reserved_name "$name"
        return 1
    fi

    _claude_validate_name "$name" || return 1

    local acc_dir="$CLAUDE_SWITCH_ACCOUNTS_DIR/$name"
    if [[ ! -d "$acc_dir" ]]; then
        _msg remove_not_found "$name"
        return 1
    fi

    if [[ "$force" != true ]]; then
        printf "$(_msg remove_confirm "$name")"
        local reply
        read -r reply
        if [[ "$reply" != [yYдД]* ]]; then
            _msg remove_cancelled
            return 1
        fi
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

    if [[ "$name" == "default" ]]; then
        sed -i '' "s/^default=.*/default=/" "$CLAUDE_SWITCH_CONFIG"
        _msg reset_done
        _claude_activate
        return
    fi

    _claude_validate_name "$name" || return 1

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

    if [[ "$name" != "default" ]]; then
        _claude_validate_name "$name" || return 1
    fi

    if [[ "$name" != "default" && ! -d "$CLAUDE_SWITCH_ACCOUNTS_DIR/$name" ]]; then
        _msg link_not_found "$name"
        _claude_acc_list
        return 1
    fi

    local dir="$PWD"

    # Убрать старую привязку для этой директории, если есть
    _claude_links_remove_dir "$dir"

    # Добавить новую
    echo "${dir}=${name}" >> "$CLAUDE_SWITCH_LINKS"
    if [[ "$name" == "default" ]]; then
        _msg link_done_default "$(basename "$dir")"
    else
        _msg link_done "$(basename "$dir")" "$name"
    fi
    _claude_activate
}

_claude_acc_unlink() {
    local dir="$PWD"

    if ! _claude_links_has_dir "$dir"; then
        _msg unlink_none
        return 1
    fi

    _claude_links_remove_dir "$dir"
    _msg unlink_done "$(basename "$dir")"
    _claude_activate
}

_claude_acc_links() {
    if [[ ! -s "$CLAUDE_SWITCH_LINKS" ]]; then
        _msg links_empty
        return
    fi

    _msg links_header

    local active_dir
    active_dir=$(_claude_find_linked_dir)

    sort "$CLAUDE_SWITCH_LINKS" | while IFS='=' read -r dir account; do
        [[ -z "$dir" || -z "$account" ]] && continue
        local display_dir="${dir/#$HOME/~}"
        if [[ "$dir" == "$active_dir" ]]; then
            echo "  $display_dir → $account  $(_msg links_active)"
        else
            echo "  $display_dir → $account"
        fi
    done
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
        local label="$account" email drift
        email=$(_claude_acc_cache_email "$account")
        if [[ -n "$email" ]]; then
            drift=$(_claude_acc_cache_drift_marker "$account")
            label="$account <${email}${drift}>"
        fi
        _msg status_active "$label" "$source_info"
    else
        _msg status_standard
    fi
}

_claude_acc_reset() {
    sed -i '' "s/^default=.*/default=/" "$CLAUDE_SWITCH_CONFIG"
    _msg reset_done
    _claude_activate
}

# Запустить claude под конкретным аккаунтом, без привязки и без смены текущей.
# `default` — запуск без CLAUDE_CONFIG_DIR (стандартный ~/.claude/).
_claude_acc_run() {
    local name="$1"
    if [[ -z "$name" ]]; then
        _msg run_usage
        return 1
    fi
    shift

    if [[ "$name" == "default" ]]; then
        command claude "$@"
        return $?
    fi

    _claude_validate_name "$name" || return 1

    local acc_dir="$CLAUDE_SWITCH_ACCOUNTS_DIR/$name"
    if [[ ! -d "$acc_dir" ]]; then
        _msg run_not_found "$name"
        return 1
    fi

    CLAUDE_CONFIG_DIR="$acc_dir" command claude "$@"
}

# --- Identity audit (Phase 1: passive, read-only) ---
# Reads each account's OAuth token from macOS Keychain (with .credentials.json
# fallback), calls /api/oauth/profile, prints email + UUID. Doesn't change
# anything. Requires: security, curl, jq, shasum.

_claude_acc_token() {
    local acc_dir="$1" hash service token
    hash=$(printf '%s' "$acc_dir" | shasum -a 256 2>/dev/null | cut -c1-8)
    [[ -z "$hash" ]] && return 1
    service="Claude Code-credentials-${hash}"
    token=$(security find-generic-password -s "$service" -a "$(id -un)" -w 2>/dev/null)
    if [[ -n "$token" ]]; then
        printf '%s' "$token" | jq -r '.claudeAiOauth.accessToken // empty' 2>/dev/null
        return 0
    fi
    jq -r '.claudeAiOauth.accessToken // empty' "$acc_dir/.credentials.json" 2>/dev/null
}

# Echoes "<email>\t<uuid>\t<org>" or empty on failure.
_claude_acc_identity() {
    local token="$1"
    [[ -z "$token" ]] && return 1
    curl -sf --max-time 5 \
        -H "Authorization: Bearer $token" \
        -H "anthropic-beta: oauth-2025-04-20" \
        https://api.anthropic.com/api/oauth/profile 2>/dev/null \
        | jq -r '"\(.account.email // "<unknown>")\t\(.account.uuid // "<unknown>")\t\(.organization.name // "")"' 2>/dev/null
}

# sha256(token)[0:16] — used as cache invalidation signal.
_claude_acc_token_hash() {
    local token="$1"
    [[ -z "$token" ]] && return 1
    printf '%s' "$token" | shasum -a 256 2>/dev/null | cut -c1-16
}

_claude_acc_cache_path() {
    echo "$1/.account-info.json"
}

_claude_acc_write_cache() {
    local acc_dir="$1" email="$2" uuid="$3" org="$4" token="$5"
    local now hash cache
    now=$(date +%s)
    hash=$(_claude_acc_token_hash "$token")
    cache=$(_claude_acc_cache_path "$acc_dir")
    jq -n \
        --arg email "$email" \
        --arg uuid "$uuid" \
        --arg org "$org" \
        --argjson fetched_at "$now" \
        --arg token_hash "$hash" \
        '{email:$email, uuid:$uuid, org:$org, fetched_at:$fetched_at, token_hash:$token_hash}' \
        > "$cache" 2>/dev/null
}

_claude_acc_cache_email() {
    local acc_dir="$CLAUDE_SWITCH_ACCOUNTS_DIR/$1"
    jq -r '.email // empty' "$(_claude_acc_cache_path "$acc_dir")" 2>/dev/null
}

# Echoes seconds elapsed since cache write, or empty if no/invalid cache.
_claude_acc_cache_age() {
    local acc_dir="$CLAUDE_SWITCH_ACCOUNTS_DIR/$1"
    local fetched now
    fetched=$(jq -r '.fetched_at // empty' "$(_claude_acc_cache_path "$acc_dir")" 2>/dev/null)
    [[ -z "$fetched" || "$fetched" == "null" ]] && return 1
    now=$(date +%s)
    (( fetched > now )) && return 1
    echo $(( now - fetched ))
}

# Prints " *" when current keychain token differs from the one cached at
# last `doctor` run. Empty otherwise (including: no cache, no current
# token, no token_hash field — i.e. when we can't compare safely).
_claude_acc_cache_drift_marker() {
    local acc_dir="$CLAUDE_SWITCH_ACCOUNTS_DIR/$1"
    local cached current
    cached=$(jq -r '.token_hash // empty' "$(_claude_acc_cache_path "$acc_dir")" 2>/dev/null)
    [[ -z "$cached" || "$cached" == "null" ]] && return 0
    current=$(_claude_acc_token_hash "$(_claude_acc_token "$acc_dir")")
    [[ -z "$current" ]] && return 0
    [[ "$cached" != "$current" ]] && printf ' *'
}

# Format a seconds count like 0/45/3600/86400 → "just now"/"45m ago"/"1h ago"/"1d ago".
# Locale follows _claude_acc_lang.
_claude_acc_relative_time() {
    local secs="$1" lang n unit
    lang=$(_claude_acc_lang)
    if (( secs < 60 )); then
        [[ "$lang" == "ru" ]] && echo "только что" || echo "just now"
        return
    fi
    if   (( secs < 3600 ));     then n=$(( secs / 60 ));      [[ "$lang" == "ru" ]] && unit="м"   || unit="m"
    elif (( secs < 86400 ));    then n=$(( secs / 3600 ));    [[ "$lang" == "ru" ]] && unit="ч"   || unit="h"
    elif (( secs < 604800 ));   then n=$(( secs / 86400 ));   [[ "$lang" == "ru" ]] && unit="д"   || unit="d"
    elif (( secs < 2592000 ));  then n=$(( secs / 604800 ));  [[ "$lang" == "ru" ]] && unit="н"   || unit="w"
    elif (( secs < 31536000 )); then n=$(( secs / 2592000 )); [[ "$lang" == "ru" ]] && unit="мес" || unit="mo"
    else                             n=$(( secs / 31536000 ));[[ "$lang" == "ru" ]] && unit="г"   || unit="y"
    fi
    if [[ "$lang" == "ru" ]]; then echo "${n}${unit} назад"; else echo "${n}${unit} ago"; fi
}

# Rendered "  email  3d ago" / "  email  3d ago *" / "" suffix for list/status.
_claude_acc_identity_suffix() {
    local name="$1" email when secs drift
    email=$(_claude_acc_cache_email "$name")
    [[ -z "$email" ]] && return 0
    secs=$(_claude_acc_cache_age "$name")
    if [[ -n "$secs" ]]; then
        when=$(_claude_acc_relative_time "$secs")
    fi
    drift=$(_claude_acc_cache_drift_marker "$name")
    if [[ -n "$when" ]]; then
        printf '  %s  %s%s' "$email" "$when" "$drift"
    else
        printf '  %s%s' "$email" "$drift"
    fi
}

_claude_acc_doctor() {
    local dep
    for dep in security curl jq shasum; do
        if ! command -v "$dep" >/dev/null 2>&1; then
            _msg doctor_missing_dep "$dep"
            return 1
        fi
    done

    local accounts=("$CLAUDE_SWITCH_ACCOUNTS_DIR"/*(N:t))
    if [[ ${#accounts} -eq 0 ]]; then
        _msg list_empty
        return 0
    fi

    _msg doctor_header "${#accounts}"

    # Compute label width for aligned output.
    local width=0 acc
    for acc in "${accounts[@]}"; do
        (( ${#acc} > width )) && width=${#acc}
    done

    local healthy=0 token identity email uuid org rest
    for acc in "${accounts[@]}"; do
        local acc_dir="$CLAUDE_SWITCH_ACCOUNTS_DIR/$acc"
        token=$(_claude_acc_token "$acc_dir")
        if [[ -z "$token" ]]; then
            printf "  ? %-${width}s  %s\n" "$acc" "$(_msg doctor_no_token "$acc")"
            continue
        fi
        identity=$(_claude_acc_identity "$token")
        if [[ -z "$identity" ]]; then
            printf "  ? %-${width}s  %s\n" "$acc" "$(_msg doctor_offline)"
            continue
        fi
        email="${identity%%	*}"
        rest="${identity#*	}"
        uuid="${rest%%	*}"
        org="${rest#*	}"
        [[ "$org" == "$rest" ]] && org=""
        _claude_acc_write_cache "$acc_dir" "$email" "$uuid" "$org" "$token"
        printf "  ✓ %-${width}s  %s  uuid=%s\n" "$acc" "$email" "$uuid"
        (( healthy++ ))
    done

    echo ""
    if (( healthy == ${#accounts} )); then
        _msg doctor_all_ok
        return 0
    else
        _msg doctor_partial "$healthy" "${#accounts}"
        return 1
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
        login)   _claude_acc_login "$@" ;;
        remove)  _claude_acc_remove "$@" ;;
        default) _claude_acc_default "$@" ;;
        reset)   _claude_acc_reset ;;
        link)    _claude_acc_link "$@" ;;
        unlink)  _claude_acc_unlink "$@" ;;
        links)   _claude_acc_links ;;
        status)  _claude_acc_status "$@" ;;
        run)     _claude_acc_run "$@" ;;
        doctor)  _claude_acc_doctor ;;
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
        "login:$(_msg help_login)"
        "remove:$(_msg help_remove)"
        "default:$(_msg help_default)"
        "reset:$(_msg help_reset)"
        "link:$(_msg help_link)"
        "unlink:$(_msg help_unlink)"
        "links:$(_msg help_links)"
        "status:$(_msg help_status)"
        "run:$(_msg help_run)"
        "doctor:$(_msg help_doctor)"
        "help:$(_msg help_help)"
    )

    if (( CURRENT == 2 )); then
        _describe 'command' subcmds
    elif (( CURRENT == 3 )); then
        case "${words[2]}" in
            login|remove|run)
                accounts=("$CLAUDE_SWITCH_ACCOUNTS_DIR"/*(N:t))
                _describe 'account' accounts
                ;;
            default|link)
                accounts=("default" "$CLAUDE_SWITCH_ACCOUNTS_DIR"/*(N:t))
                _describe 'account' accounts
                ;;
        esac
    fi
}

compdef _claude_acc_completion claude-acc

# =============================================================
# PATH: поставить wrapper впереди настоящего claude
# =============================================================
# Добавляем только если ещё не в PATH, чтобы не дублировать при повторном source.
if [[ ":$PATH:" != *":$CLAUDE_SWITCH_BIN:"* ]]; then
    export PATH="$CLAUDE_SWITCH_BIN:$PATH"
fi
