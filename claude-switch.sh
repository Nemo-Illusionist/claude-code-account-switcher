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
CLAUDE_SWITCH_DESKTOP_DIR="$CLAUDE_SWITCH_DIR/desktop"
CLAUDE_SWITCH_SCRIPT="${(%):-%x}"
CLAUDE_SWITCH_SCRIPT="${CLAUDE_SWITCH_SCRIPT:A}"

# =============================================================
# Localization / i18n
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
    help_usage          "Show 5h / 7d usage for every account"
    help_update         "Update claude-switch.sh from GitHub"
    help_run            "Run claude under a specific account"
    help_doctor         "Audit each account OAuth identity (email, UUID)"
    help_whoami         "Print active account email (or name fallback)"
    help_clone_settings "Copy ~/.claude/ config into an existing account"
    help_import         "Import an existing Claude config dir (no re-login)"
    help_help           "Help"
    list_empty          "No accounts. Add one: claude-acc add <name>"
    list_header         "Claude Code accounts:"
    list_default        "(default)"
    list_standard       "(standard)"
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
    seed_copied         "  copied: %s"
    seed_nothing        "  nothing to copy from ~/.claude/"
    clone_settings_usage "Usage: claude-acc clone-settings <name>"
    import_usage        "Usage: claude-acc import <name> <path> [--move]"
    import_source_not_dir "Source '%s' is not a directory."
    import_source_managed "That path is already inside ~/.claude-switch/accounts/."
    import_failed       "Import failed: %s"
    import_done         "Imported '%s' → %s"
    import_rekeyed      "Moved the Keychain token to the new location."
    import_verified     "Verified identity: %s"
    run_usage           "Usage: claude-acc run <name> [args...]"
    run_not_found       "Account '%s' not found."
    doctor_header       "Auditing %d account(s):"
    doctor_no_token     "no token (run: claude-acc login %s)"
    doctor_offline      "token present, but API unreachable"
    doctor_all_ok       "All accounts healthy."
    doctor_partial      "%d of %d accounts healthy."
    doctor_shared_identity "↔ same identity as %s"
    doctor_missing_dep  "claude-acc doctor needs '%s' on PATH."
    unlink_none         "No link for the current directory."
    unlink_done         "Unlinked %s. Default account will be used."
    status_active       "Active account: %s %s"
    status_linked       "(linked to %s)"
    status_default      "(default)"
    status_standard     "Active account: ~/.claude/ (standard)"
    usage_header        "Claude Code usage:"
    usage_resets_in     "resets in %s"
    usage_available_now "available now"
    usage_missing_dep   "claude-acc usage needs '%s' on PATH."
    update_fetching     "Fetching latest claude-switch.sh..."
    update_download_failed "Download failed."
    update_script_unknown  "Can't locate the installed claude-switch.sh."
    update_done         "Updated %s. Re-source it: source %s"
    update_missing_dep  "claude-acc update needs '%s' on PATH."
    links_empty         "No links. Use: claude-acc link <name>"
    links_header        "Links:"
    links_active        "← active"
    help_desktop        "Manage Claude Desktop profiles"
    desktop_usage       "Usage: claude-acc desktop add|clone-config|clone-runtime|list|run|remove [<name>]"
    desktop_usage_rust  "claude-acc desktop usage needs the Rust CLI — decrypting a profile token needs PBKDF2-SHA1 + AES, which stock macOS openssl can't do."
    desktop_app_not_found "Claude.app not found in /Applications or ~/Applications. Set CLAUDE_ACC_DESKTOP_APP to its path."
    desktop_no_default  "'default' isn't a desktop profile — that's the app's own, which you open as usual."
    desktop_exists      "Desktop profile '%s' already exists."
    desktop_not_found   "Desktop profile '%s' not found. Create it: claude-acc desktop add %s"
    desktop_created     "Desktop profile '%s' created. Opening Claude on it..."
    desktop_signin_hint "It opens signed out — sign in there with the account for this profile."
    desktop_quit_first  "Quit Claude before signing in. Signing in finishes through a claude:// link, and the system hands that to whichever instance is registered for it — with another one open, the new profile never receives it. Currently open:"
    desktop_running_instance "  pid %s  —  %s"
    desktop_quit_first_hint "Quit them, sign in, and after that profiles open side by side as usual. To go ahead anyway: --force"
    desktop_signin_forced "Claude is already running; --force given, so the sign-in may not arrive."
    desktop_disk_note   "The profile is fully isolated, so the app would re-download its whole runtime into it — about 10.5 GB. To skip that, clone it instead (free, on APFS):"
    desktop_disk_hint   "  claude-acc desktop clone-runtime %s"
    desktop_runtime_no_source "%s has no downloaded runtime to clone."
    desktop_runtime_keep "This profile already has a downloaded runtime. Replace it with --force."
    desktop_runtime_would_copy "The profile is on a different filesystem from the source, so this would copy every byte instead of cloning it — the opposite of the point. Not done."
    desktop_runtime_cloned "Cloned %s runtime component(s) from %s."
    desktop_runtime_unverified "Whether the app accepts a pre-seeded runtime is untested — if this profile misbehaves, delete its vm_bundles/, claude-code/ and claude-code-vm/, and it will fetch its own."
    desktop_runtime_failed "Could not clone the runtime."
    desktop_hint_run    "Open it again later:  claude-acc desktop run %s"
    desktop_list_empty  "No desktop profiles. Add one: claude-acc desktop add <name>"
    desktop_list_header "Claude Desktop profiles:"
    desktop_signed_in   "(signed in)"
    desktop_signed_out  "(signed out)"
    desktop_standard    "(the app's own profile)"
    desktop_launching   "Opening Claude on profile '%s'..."
    desktop_launch_failed "Could not open Claude."
    desktop_remove_warn "This deletes the whole profile '%s' — its sign-in, its settings and its MCP servers. Quit that window first."
    desktop_remove_confirm "Delete profile '%s'? [y/N] "
    desktop_remove_cancelled "Cancelled."
    desktop_removed     "Desktop profile '%s' deleted."
    desktop_clone_no_source "%s has no claude_desktop_config.json to copy."
    desktop_clone_keep  "This profile already has a claude_desktop_config.json. Replace it with --force."
    desktop_clone_done  "MCP servers and preferences copied from %s."
    desktop_clone_auth_note "Server definitions only — any that sign in separately will ask for that again in the new profile."
    desktop_clone_failed "Could not copy the config."
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
    help_usage          "Показать использование 5ч / 7д по всем аккаунтам"
    help_update         "Обновить claude-switch.sh с GitHub"
    help_run            "Запустить claude под конкретным аккаунтом"
    help_doctor         "Аудит OAuth-личности каждого аккаунта (email, UUID)"
    help_whoami         "Email активного аккаунта (или имя как fallback)"
    help_clone_settings "Скопировать конфиг ~/.claude/ в существующий аккаунт"
    help_import         "Импортировать существующую config-папку (без релогина)"
    help_help           "Справка"
    list_empty          "Нет аккаунтов. Добавьте: claude-acc add <name>"
    list_header         "Аккаунты Claude Code:"
    list_default        "(по умолчанию)"
    list_standard       "(стандартный)"
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
    seed_copied         "  скопировано: %s"
    seed_nothing        "  нечего копировать из ~/.claude/"
    clone_settings_usage "Использование: claude-acc clone-settings <name>"
    import_usage        "Использование: claude-acc import <name> <путь> [--move]"
    import_source_not_dir "Источник '%s' не является директорией."
    import_source_managed "Этот путь уже внутри ~/.claude-switch/accounts/."
    import_failed       "Импорт не удался: %s"
    import_done         "Импортирован '%s' → %s"
    import_rekeyed      "Токен Keychain перенесён на новое место."
    import_verified     "Личность подтверждена: %s"
    run_usage           "Использование: claude-acc run <name> [args...]"
    run_not_found       "Аккаунт '%s' не найден."
    doctor_header       "Проверка %d аккаунт(ов):"
    doctor_no_token     "нет токена (запустите: claude-acc login %s)"
    doctor_offline      "токен есть, API недоступен"
    doctor_all_ok       "Все аккаунты в порядке."
    doctor_partial      "%d из %d аккаунтов в порядке."
    doctor_shared_identity "↔ та же личность, что и %s"
    doctor_missing_dep  "claude-acc doctor требует '%s' в PATH."
    unlink_none         "Нет привязки для текущей директории."
    unlink_done         "Привязка убрана для %s. Будет использован дефолтный аккаунт."
    status_active       "Активный аккаунт: %s %s"
    status_linked       "(привязан к %s)"
    status_default      "(по умолчанию)"
    status_standard     "Активный аккаунт: ~/.claude/ (стандартный)"
    usage_header        "Использование Claude Code:"
    usage_resets_in     "сброс через %s"
    usage_available_now "доступно сейчас"
    usage_missing_dep   "claude-acc usage требует '%s' в PATH."
    update_fetching     "Скачиваю свежий claude-switch.sh..."
    update_download_failed "Не удалось скачать."
    update_script_unknown  "Не удалось найти установленный claude-switch.sh."
    update_done         "Обновлено: %s. Перезагрузите: source %s"
    update_missing_dep  "claude-acc update требует '%s' в PATH."
    links_empty         "Нет привязок. Используйте: claude-acc link <name>"
    links_header        "Привязки:"
    links_active        "← активна"
    help_desktop        "Профили Claude Desktop"
    desktop_usage       "Использование: claude-acc desktop add|clone-config|clone-runtime|list|run|remove [<name>]"
    desktop_usage_rust  "claude-acc desktop usage есть только в Rust CLI — расшифровка токена профиля требует PBKDF2-SHA1 + AES, чего стоковый openssl macOS не умеет."
    desktop_app_not_found "Claude.app не найден в /Applications или ~/Applications. Укажите путь в CLAUDE_ACC_DESKTOP_APP."
    desktop_no_default  "'default' — не профиль десктопа: это собственный профиль приложения, откройте его как обычно."
    desktop_exists      "Профиль десктопа '%s' уже существует."
    desktop_not_found   "Профиль десктопа '%s' не найден. Создать: claude-acc desktop add %s"
    desktop_created     "Профиль десктопа '%s' создан. Открываю Claude на нём..."
    desktop_signin_hint "Оно откроется без входа — войдите там под аккаунтом для этого профиля."
    desktop_quit_first  "Закройте Claude перед входом. Вход завершается переходом по ссылке claude://, а её система отдаёт тому экземпляру, что зарегистрирован на неё, — при другом открытом окне новый профиль её просто не получит. Сейчас открыто:"
    desktop_running_instance "  pid %s  —  %s"
    desktop_quit_first_hint "Закройте их, войдите — после этого профили открываются одновременно как обычно. Всё равно продолжить: --force"
    desktop_signin_forced "Claude уже запущен; передан --force, так что вход может не дойти."
    desktop_disk_note   "Профиль полностью изолирован, поэтому приложение заново скачало бы в него весь рантайм — около 10.5 ГБ. Чтобы этого избежать, склонируйте его (на APFS это бесплатно):"
    desktop_disk_hint   "  claude-acc desktop clone-runtime %s"
    desktop_runtime_no_source "В %s нет скачанного рантайма — клонировать нечего."
    desktop_runtime_keep "У этого профиля уже есть скачанный рантайм. Заменить — с --force."
    desktop_runtime_would_copy "Профиль лежит на другой файловой системе, чем источник, поэтому вместо клонирования скопировался бы каждый байт — ровно наоборот от смысла. Не выполнено."
    desktop_runtime_cloned "Склонировано компонентов рантайма: %s, из %s."
    desktop_runtime_unverified "Примет ли приложение подложенный рантайм — не проверено: если профиль поведёт себя странно, удалите его vm_bundles/, claude-code/ и claude-code-vm/, и он скачает свои."
    desktop_runtime_failed "Не удалось склонировать рантайм."
    desktop_hint_run    "Открыть его позже:  claude-acc desktop run %s"
    desktop_list_empty  "Нет профилей десктопа. Добавьте: claude-acc desktop add <name>"
    desktop_list_header "Профили Claude Desktop:"
    desktop_signed_in   "(выполнен вход)"
    desktop_signed_out  "(вход не выполнен)"
    desktop_standard    "(собственный профиль приложения)"
    desktop_launching   "Открываю Claude на профиле '%s'..."
    desktop_launch_failed "Не удалось открыть Claude."
    desktop_remove_warn "Профиль '%s' будет удалён целиком — вход, настройки и MCP-серверы. Сначала закройте его окно."
    desktop_remove_confirm "Удалить профиль '%s'? [y/N] "
    desktop_remove_cancelled "Отменено."
    desktop_removed     "Профиль десктопа '%s' удалён."
    desktop_clone_no_source "В %s нет claude_desktop_config.json — копировать нечего."
    desktop_clone_keep  "У этого профиля уже есть claude_desktop_config.json. Заменить — с --force."
    desktop_clone_done  "MCP-серверы и настройки скопированы из %s."
    desktop_clone_auth_note "Скопированы только описания серверов — те, что логинятся отдельно, попросят вход и в новом профиле."
    desktop_clone_failed "Не удалось скопировать конфиг."
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
# Core
# =============================================================

# --- Account name validation ---
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

# --- Init ---
_claude_switch_init() {
    mkdir -p "$CLAUDE_SWITCH_ACCOUNTS_DIR"
    mkdir -p "$CLAUDE_SWITCH_BIN"
    [[ -f "$CLAUDE_SWITCH_CONFIG" ]] || echo "default=" > "$CLAUDE_SWITCH_CONFIG"
    [[ -f "$CLAUDE_SWITCH_LINKS" ]]  || touch "$CLAUDE_SWITCH_LINKS"
    # Migration: rename repos → links
    if [[ -f "$CLAUDE_SWITCH_DIR/repos" && ! -s "$CLAUDE_SWITCH_LINKS" ]]; then
        mv "$CLAUDE_SWITCH_DIR/repos" "$CLAUDE_SWITCH_LINKS"
    fi
    # Set up ide/ symlinks and the IDE-integration wrapper
    _claude_ensure_ide_symlinks
    _claude_ensure_wrapper
}

# --- Create ide/ symlinks for every account ---
# Every account points at ~/.claude/ide/ so IDE plugins (PhpStorm, VSCode)
# always find Claude Code's lock files in the canonical location.
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

# --- Create wrapper script ~/.claude-switch/bin/claude ---
# IDEs (PhpStorm etc.) launch claude directly without sourcing .zshrc, so
# CLAUDE_CONFIG_DIR wouldn't be set. The wrapper sources claude-switch.sh
# itself — that runs _claude_activate against $PWD and reuses the same
# lookup logic as the terminal chpwd hook (no code duplication).
_claude_ensure_wrapper() {
    local wrapper="$CLAUDE_SWITCH_BIN/claude"

    # Regenerate only if wrapper is missing or older than the main script
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

# --- Safe operations on the links file ---
# Pure-shell processing instead of grep/sed for two reasons:
# 1. Paths can contain regex metacharacters (`.`, `[`, `+` etc. — common
#    on macOS) which break anchored-grep with over-matching.
# 2. `sed -i ''` for line deletion also treats the pattern as regex.
# `[[ "$line" == "${dir}="* ]]` uses a glob pattern with a literal prefix
# and a single trailing `*` — no surprises with metacharacters in `$dir`.

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

# --- Read configured default account ---
_claude_default_account() {
    grep '^default=' "$CLAUDE_SWITCH_CONFIG" 2>/dev/null | cut -d= -f2
}

# --- Look up account for a directory (exact match) ---
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

# --- Resolve account by walking up the directory tree ---
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

# --- Find the directory that owns the active link (for status) ---
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

# --- Set CLAUDE_CONFIG_DIR for the current context ---
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

# --- chpwd hook: auto-switch account on cd ---
_claude_chpwd_hook() {
    _claude_activate
}

# Register the hook (zsh calls chpwd on every directory change)
autoload -Uz add-zsh-hook
add-zsh-hook chpwd _claude_chpwd_hook

# Activate immediately for the current directory
_claude_activate

# =============================================================
# Subcommands
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
    echo "  claude-acc usage             $(_msg help_usage)"
    echo "  claude-acc update            $(_msg help_update)"
    echo "  claude-acc run <name> [...]  $(_msg help_run)"
    echo "  claude-acc doctor [--json]   $(_msg help_doctor)"
    echo "  claude-acc whoami            $(_msg help_whoami)"
    echo "  claude-acc add -s <name>     $(_msg help_add) (seeded from ~/.claude/)"
    echo "  claude-acc clone-settings <name>  $(_msg help_clone_settings)"
    echo "  claude-acc import <name> <path>   $(_msg help_import)"
    echo "  claude-acc desktop add|clone-config|clone-runtime|list|run|remove [<name>]  $(_msg help_desktop)"
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

    # Unmanaged ~/.claude/ — show row only if there's a real keychain login
    # there OR an existing default cache (i.e. doctor has audited it before).
    local std_token std_cache
    std_token=$(_claude_acc_token "$(_claude_acc_default_token_dir)")
    std_cache=$(_claude_acc_default_cache_path)
    if [[ -n "$std_token" || -f "$std_cache" ]]; then
        suffix=$(_claude_acc_default_suffix)
        echo "    ~/.claude/$suffix  $(_msg list_standard)"
    fi
}

_claude_acc_add() {
    local seed_flag=0 name=""
    while (( $# > 0 )); do
        case "$1" in
            -s|--seed) seed_flag=1; shift ;;
            *)         name="$1"; shift ;;
        esac
    done

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
    (( seed_flag )) && _claude_acc_seed_from_default "$acc_dir"
    _msg add_created "$name"
    # A leaked ANTHROPIC_API_KEY etc. can make `claude auth login` skip the
    # OAuth flow entirely, or auth a different identity than acc_dir intends.
    (
        unset ANTHROPIC_API_KEY ANTHROPIC_AUTH_TOKEN \
            CLAUDE_CODE_OAUTH_TOKEN AWS_BEARER_TOKEN_BEDROCK
        CLAUDE_CONFIG_DIR="$acc_dir" claude auth login
    )
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

    if [[ "$name" == "default" ]]; then
        _msg login_start "$name"
        # Clear any inherited CLAUDE_CONFIG_DIR and tell claude-acc's IDE
        # wrapper not to re-derive one from $PWD — same reasoning as `run
        # default`. No keychain snapshot/restore: unlike logging into a
        # *different* account, this login is meant to change the standard
        # account's own credentials.
        (
            unset CLAUDE_CONFIG_DIR ANTHROPIC_API_KEY ANTHROPIC_AUTH_TOKEN \
                CLAUDE_CODE_OAUTH_TOKEN AWS_BEARER_TOKEN_BEDROCK
            CLAUDE_ACC_RUN_DEFAULT=1 claude auth login
        )
        _msg login_done
        return
    fi

    _claude_validate_name "$name" || return 1

    local acc_dir="$CLAUDE_SWITCH_ACCOUNTS_DIR/$name"
    if [[ ! -d "$acc_dir" ]]; then
        _msg login_not_found "$name"
        return 1
    fi

    _msg login_start "$name"
    # A leaked ANTHROPIC_API_KEY etc. can make `claude auth login` skip the
    # OAuth flow entirely, or auth a different identity than acc_dir intends.
    (
        unset ANTHROPIC_API_KEY ANTHROPIC_AUTH_TOKEN \
            CLAUDE_CODE_OAUTH_TOKEN AWS_BEARER_TOKEN_BEDROCK
        CLAUDE_CONFIG_DIR="$acc_dir" claude auth login
    )
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

    # Clear default if it was this account
    local default_acc
    default_acc=$(_claude_default_account)
    if [[ "$default_acc" == "$name" ]]; then
        sed -i '' "s/^default=.*/default=/" "$CLAUDE_SWITCH_CONFIG"
    fi

    # Drop links pointing at this account
    sed -i '' "/=$name$/d" "$CLAUDE_SWITCH_LINKS"

    rm -rf "$acc_dir"
    _msg remove_deleted "$name"
    _claude_activate
}

_claude_acc_default() {
    local name="$1"
    if [[ -z "$name" ]]; then
        local current email label
        current=$(_claude_default_account)
        if [[ -n "$current" ]]; then
            label="$current"
            email=$(_claude_acc_cache_email "$current")
            [[ -n "$email" ]] && label="$current <$email>"
            _msg default_current "$label"
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

    # Drop existing link for this directory, if any
    _claude_links_remove_dir "$dir"

    # Append the new one
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
        # Standard ~/.claude/ — surface email if `doctor` has cached one.
        local std_email std_drift
        std_email=$(_claude_acc_default_email)
        if [[ -n "$std_email" ]]; then
            std_drift=$(_claude_acc_default_drift_marker)
            _msg status_active "~/.claude/ <${std_email}${std_drift}>" "$(_msg list_standard)"
        else
            _msg status_standard
        fi
    fi
}

_claude_acc_reset() {
    sed -i '' "s/^default=.*/default=/" "$CLAUDE_SWITCH_CONFIG"
    _msg reset_done
    _claude_activate
}

# Seed an account dir with the user's standard ~/.claude/ config (curated set).
# Skips files that already exist. Mirrors `seed::copy_user_config` in the
# Rust binary — see src/seed.rs for the rationale on what's copied / not.
_claude_acc_seed_from_default() {
    local target="$1"
    local source="$HOME/.claude"
    [[ ! -d "$source" ]] && return 0

    local files=(settings.json CLAUDE.md)
    local dirs=(agents commands output-styles skills)
    local any=0 f d count

    for f in "${files[@]}"; do
        if [[ -f "$source/$f" && ! -e "$target/$f" ]]; then
            cp "$source/$f" "$target/$f"
            _msg seed_copied "$f"
            any=1
        fi
    done

    for d in "${dirs[@]}"; do
        if [[ -d "$source/$d" && ! -e "$target/$d" ]]; then
            # Skip empty source dirs.
            count=$(find "$source/$d" -type f 2>/dev/null | wc -l | tr -d ' ')
            (( count == 0 )) && continue
            cp -R "$source/$d" "$target/$d"
            local plural=""
            (( count != 1 )) && plural="s"
            _msg seed_copied "$d/ ($count file$plural)"
            any=1
        fi
    done

    (( any == 0 )) && _msg seed_nothing
    return 0
}

_claude_acc_clone_settings() {
    local name="$1"
    if [[ -z "$name" ]]; then
        _msg clone_settings_usage
        return 1
    fi
    if [[ "$name" == "default" ]]; then
        _msg reserved_name "$name"
        return 1
    fi
    _claude_validate_name "$name" || return 1
    local acc_dir="$CLAUDE_SWITCH_ACCOUNTS_DIR/$name"
    if [[ ! -d "$acc_dir" ]]; then
        _msg login_not_found "$name"
        return 1
    fi
    _claude_acc_seed_from_default "$acc_dir"
}

# Run claude under a specific account — one-shot, doesn't change links or
# the current shell's active account. `default` runs without
# CLAUDE_CONFIG_DIR (standard ~/.claude/).
_claude_acc_run() {
    local name="$1"
    if [[ -z "$name" ]]; then
        _msg run_usage
        return 1
    fi
    shift

    # Any of these can override which identity claude actually uses
    # regardless of CLAUDE_CONFIG_DIR, if they leaked in from the parent
    # shell — strip them so `run <name>` can't be silently overridden.
    if [[ "$name" == "default" ]]; then
        # CLAUDE_ACC_RUN_DEFAULT tells claude-acc's own IDE wrapper (which
        # `claude` usually resolves to on PATH) not to re-derive
        # CLAUDE_CONFIG_DIR from $PWD — otherwise it would silently undo
        # this explicit default run inside a linked directory.
        (
            unset CLAUDE_CONFIG_DIR ANTHROPIC_API_KEY ANTHROPIC_AUTH_TOKEN \
                CLAUDE_CODE_OAUTH_TOKEN AWS_BEARER_TOKEN_BEDROCK
            CLAUDE_ACC_RUN_DEFAULT=1 command claude "$@"
        )
        return $?
    fi

    _claude_validate_name "$name" || return 1

    local acc_dir="$CLAUDE_SWITCH_ACCOUNTS_DIR/$name"
    if [[ ! -d "$acc_dir" ]]; then
        _msg run_not_found "$name"
        return 1
    fi

    (
        unset ANTHROPIC_API_KEY ANTHROPIC_AUTH_TOKEN \
            CLAUDE_CODE_OAUTH_TOKEN AWS_BEARER_TOKEN_BEDROCK
        CLAUDE_CONFIG_DIR="$acc_dir" command claude "$@"
    )
}

# =============================================================
# Claude Desktop profiles
# =============================================================
# The desktop app is Electron, so it honours Chromium's --user-data-dir.
# Pointing it at a directory of our own gives a fully isolated app profile —
# the same move this tool makes for the CLI with CLAUDE_CONFIG_DIR. Unlike CLI
# accounts these run side by side: the app takes no single-instance lock.

_claude_acc_desktop_app() {
    if [[ -n "$CLAUDE_ACC_DESKTOP_APP" ]]; then
        # A set-but-wrong override resolves to nothing rather than falling
        # back, so a typo is visible instead of silently opening the app the
        # override meant to replace.
        [[ -e "$CLAUDE_ACC_DESKTOP_APP" ]] && echo "$CLAUDE_ACC_DESKTOP_APP"
        return
    fi
    local candidate
    for candidate in "/Applications/Claude.app" "$HOME/Applications/Claude.app"; do
        if [[ -e "$candidate" ]]; then
            echo "$candidate"
            return
        fi
    done
}

_claude_acc_desktop_launch() {
    local app="$1" profile="$2"
    # -n starts a new process instead of activating the running one;
    # everything after --args is handed to the app itself.
    open -n -a "$app" --args "--user-data-dir=$profile" || {
        _msg desktop_launch_failed
        return 1
    }
}

# Only the presence of a credential is checked, not its validity. The pre-V2
# key survives the migration as an empty string, hence the non-empty match.
_claude_acc_desktop_signed_in() {
    local profile="$1"
    grep -q '"oauth:tokenCache\(V2\)\?":"[^"]' "$profile/config.json" 2>/dev/null
}

# The account uuid sits in plaintext in config.json — enough to tell two
# profiles apart without touching the keychain. The email needs the token
# decrypted, which is Rust-only; see desktop_usage_rust.
_claude_acc_desktop_uuid() {
    local profile="$1" uuid
    [[ -f "$profile/config.json" ]] || return
    if command -v jq >/dev/null 2>&1; then
        uuid=$(jq -r '.lastKnownAccountUuid // empty' "$profile/config.json" 2>/dev/null)
        [[ -n "$uuid" ]] && echo "  ${uuid:0:8}…"
    fi
}

# Signing in only works with no other Claude open: the claude:// callback
# that finishes it goes to whichever instance is registered for the scheme.
# Every Chromium child carries --type=, which is what separates the app from
# its helpers — the executable paths don't.
_claude_acc_desktop_signin_possible() {
    local force="$1" line pid rest exe profile
    local -a running lines
    lines=(${(f)"$(ps -ax -o pid=,command=)"})
    for line in "${lines[@]}"; do
        # `read` does the leading-whitespace trimming, so nothing here needs
        # extended glob — which is off in a bare shell and can't be relied on.
        read -r pid rest <<< "$line"
        # The first token must be the app's executable: matching the line
        # anywhere would count any process that merely names the path. The
        # helpers live at ".../Frameworks/Claude Helper.app/...", whose first
        # token stops at that space, and every one of them carries --type=.
        exe="${rest%% *}"
        [[ "$exe" == */Contents/MacOS/Claude ]] || continue
        [[ "$rest" == *--type=* ]] && continue
        running+=("$pid $rest")
    done
    (( ${#running} == 0 )) && return 0
    if [[ "$force" == true ]]; then
        _msg desktop_signin_forced
        return 0
    fi
    _msg desktop_quit_first
    for line in "${running[@]}"; do
        read -r pid rest <<< "$line"
        if [[ "$rest" == *--user-data-dir=* ]]; then
            profile="${rest##*--user-data-dir=}"
            profile="${${profile%% *}:t}"
        else
            profile="$CLAUDE_DESKTOP_STANDARD_LABEL"
        fi
        _msg desktop_running_instance "$pid" "$profile"
    done
    _msg desktop_quit_first_hint
    return 1
}

_claude_acc_desktop_name_ok() {
    local name="$1"
    if [[ -z "$name" ]]; then
        _msg desktop_usage
        return 1
    fi
    if [[ "$name" == "default" ]]; then
        _msg desktop_no_default
        return 1
    fi
    _claude_validate_name "$name"
}

_claude_acc_desktop_add() {
    local seed_flag=0 force=false name=""
    while (( $# > 0 )); do
        case "$1" in
            -s|--seed) seed_flag=1; shift ;;
            --force)   force=true; shift ;;
            *)         name="$1"; shift ;;
        esac
    done
    _claude_acc_desktop_name_ok "$name" || return 1

    local profile="$CLAUDE_SWITCH_DESKTOP_DIR/$name"
    if [[ -d "$profile" ]]; then
        _msg desktop_exists "$name"
        return 1
    fi

    # Checked before the directory is created, so a refusal leaves nothing
    # half-made behind.
    _claude_acc_desktop_signin_possible "$force" || return 1

    local app
    app=$(_claude_acc_desktop_app)
    if [[ -z "$app" ]]; then
        _msg desktop_app_not_found
        return 1
    fi

    mkdir -p "$profile" || return 1
    _msg desktop_created "$name"
    # A fresh profile has no config, so nothing can be overwritten — and a
    # failed seed is not a reason to withhold the profile itself.
    (( seed_flag )) && _claude_acc_desktop_seed "$profile" "" false
    _msg desktop_signin_hint
    _msg desktop_disk_note
    _msg desktop_disk_hint "$name"
    _claude_acc_desktop_launch "$app" "$profile" || return 1
    echo ""
    _msg desktop_hint_run "$name"
}

_claude_acc_desktop_list() {
    local profiles=("$CLAUDE_SWITCH_DESKTOP_DIR"/*(N/:t))
    if [[ ${#profiles} -eq 0 ]]; then
        _msg desktop_list_empty
        return
    fi

    _msg desktop_list_header
    local name state
    for name in "${profiles[@]}"; do
        if _claude_acc_desktop_signed_in "$CLAUDE_SWITCH_DESKTOP_DIR/$name"; then
            state=$(_msg desktop_signed_in)
        else
            state=$(_msg desktop_signed_out)
        fi
        echo "    $name$(_claude_acc_desktop_uuid "$CLAUDE_SWITCH_DESKTOP_DIR/$name")  $state"
    done

    # The app's own profile, so the list reads as the full picture rather
    # than only the part this tool created.
    if [[ -d "$HOME/Library/Application Support/Claude" ]]; then
        echo "    ~/Library/…/Claude/  $(_msg desktop_standard)"
    fi
}

_claude_acc_desktop_run() {
    local force=false name=""
    while (( $# > 0 )); do
        case "$1" in
            --force) force=true; shift ;;
            *)       name="$1"; shift ;;
        esac
    done
    _claude_acc_desktop_name_ok "$name" || return 1

    local profile="$CLAUDE_SWITCH_DESKTOP_DIR/$name"
    if [[ ! -d "$profile" ]]; then
        _msg desktop_not_found "$name" "$name"
        return 1
    fi

    # A profile with no credential is about to be signed in, whether or not
    # the user thinks of it that way. One already signed in needs no clear
    # field — instances coexist fine after that.
    if ! _claude_acc_desktop_signed_in "$profile"; then
        _claude_acc_desktop_signin_possible "$force" || return 1
    fi

    local app
    app=$(_claude_acc_desktop_app)
    if [[ -z "$app" ]]; then
        _msg desktop_app_not_found
        return 1
    fi

    _msg desktop_launching "$name"
    _claude_acc_desktop_launch "$app" "$profile"
}

_claude_acc_desktop_remove() {
    local force=false
    if [[ "$1" == "-f" || "$1" == "--force" ]]; then
        force=true
        shift
    fi

    local name="$1"
    _claude_acc_desktop_name_ok "$name" || return 1

    local profile="$CLAUDE_SWITCH_DESKTOP_DIR/$name"
    if [[ ! -d "$profile" ]]; then
        _msg desktop_not_found "$name" "$name"
        return 1
    fi

    if [[ "$force" != true ]]; then
        _msg desktop_remove_warn "$name"
        printf "$(_msg desktop_remove_confirm "$name")"
        local reply
        read -r reply
        if [[ "$reply" != [yYдД]* ]]; then
            _msg desktop_remove_cancelled
            return 1
        fi
    fi

    rm -rf "$profile"
    _msg desktop_removed "$name"
}

# --- Seed a profile's MCP servers / preferences ---
# The file lives inside the user-data dir, so a new profile starts without
# one. Copying is via a staging file so an interrupted copy can't leave the
# destination holding half a config; cp preserves the source mode, which
# matters because the file can hold MCP credentials and is 0600.
CLAUDE_DESKTOP_CONFIG_FILE="claude_desktop_config.json"
CLAUDE_DESKTOP_STANDARD_LABEL="~/Library/…/Claude/"

_claude_acc_desktop_seed() {
    local profile="$1" from="$2" force="$3"
    local source label

    if [[ -n "$from" ]]; then
        _claude_acc_desktop_name_ok "$from" || return 1
        source="$CLAUDE_SWITCH_DESKTOP_DIR/$from"
        if [[ ! -d "$source" ]]; then
            _msg desktop_not_found "$from" "$from"
            return 1
        fi
        label="$from"
    else
        source="$HOME/Library/Application Support/Claude"
        label="$CLAUDE_DESKTOP_STANDARD_LABEL"
    fi

    if [[ ! -f "$source/$CLAUDE_DESKTOP_CONFIG_FILE" ]]; then
        _msg desktop_clone_no_source "$label"
        return 1
    fi
    if [[ -f "$profile/$CLAUDE_DESKTOP_CONFIG_FILE" && "$force" != true ]]; then
        _msg desktop_clone_keep
        return 1
    fi

    local staged="$profile/$CLAUDE_DESKTOP_CONFIG_FILE.part"
    rm -f "$staged"
    if ! cp -p "$source/$CLAUDE_DESKTOP_CONFIG_FILE" "$staged"; then
        _msg desktop_clone_failed
        return 1
    fi
    mv -f "$staged" "$profile/$CLAUDE_DESKTOP_CONFIG_FILE" || {
        _msg desktop_clone_failed
        return 1
    }
    _msg desktop_clone_done "$label"
    _msg desktop_clone_auth_note
}

_claude_acc_desktop_clone_config() {
    local from="" force=false
    local -a rest
    while (( $# > 0 )); do
        case "$1" in
            --from)        from="$2"; shift 2 ;;
            -f|--force)    force=true; shift ;;
            *)             rest+=("$1"); shift ;;
        esac
    done

    local name="${rest[1]}"
    _claude_acc_desktop_name_ok "$name" || return 1

    local profile="$CLAUDE_SWITCH_DESKTOP_DIR/$name"
    if [[ ! -d "$profile" ]]; then
        _msg desktop_not_found "$name" "$name"
        return 1
    fi
    _claude_acc_desktop_seed "$profile" "$from" "$force"
}

# --- Share the downloaded runtime between profiles ---
# Most of a profile's weight is components the app downloads and then only
# reads: ~10 GB of Cowork sandbox images plus the embedded Claude Code and its
# VM. APFS clones them copy-on-write, so a second profile's copy costs nothing
# until it diverges — safer than sharing one file between two live VMs.
# Per-VM identity (macAddress, machineIdentifier, sessiondata) is deliberately
# left behind; the app makes its own. Live caches are never cloned.
CLAUDE_DESKTOP_BUNDLE="vm_bundles/claudevm.bundle"
typeset -ga CLAUDE_DESKTOP_BUNDLE_FILES CLAUDE_DESKTOP_VERSIONED_ROOTS
CLAUDE_DESKTOP_BUNDLE_FILES=(
    rootfs.img vmlinuz initrd initrd-micro initrd-micro.zst
    .rootfs.img.origin .vmlinuz.origin .initrd.origin
    .initrd-micro.origin .initrd-micro.zst.origin
)
CLAUDE_DESKTOP_VERSIONED_ROOTS=(claude-code claude-code-vm)

# Paths, relative to a profile, that are worth cloning.
_claude_acc_desktop_runtime_items() {
    local profile="$1" f root entry
    for f in "${CLAUDE_DESKTOP_BUNDLE_FILES[@]}"; do
        [[ -f "$profile/$CLAUDE_DESKTOP_BUNDLE/$f" ]] && echo "$CLAUDE_DESKTOP_BUNDLE/$f"
    done
    for root in "${CLAUDE_DESKTOP_VERSIONED_ROOTS[@]}"; do
        [[ -d "$profile/$root" ]] || continue
        for entry in "$profile/$root"/*(ND); do
            [[ "${entry:t}" == ".DS_Store" ]] && continue
            echo "$root/${entry:t}"
        done
    done
}

_claude_acc_desktop_clone_runtime() {
    local from="" force=false
    local -a rest
    while (( $# > 0 )); do
        case "$1" in
            --from)     from="$2"; shift 2 ;;
            -f|--force) force=true; shift ;;
            *)          rest+=("$1"); shift ;;
        esac
    done

    local name="${rest[1]}"
    _claude_acc_desktop_name_ok "$name" || return 1

    local profile="$CLAUDE_SWITCH_DESKTOP_DIR/$name"
    if [[ ! -d "$profile" ]]; then
        _msg desktop_not_found "$name" "$name"
        return 1
    fi

    local source label
    if [[ -n "$from" ]]; then
        _claude_acc_desktop_name_ok "$from" || return 1
        source="$CLAUDE_SWITCH_DESKTOP_DIR/$from"
        if [[ ! -d "$source" ]]; then
            _msg desktop_not_found "$from" "$from"
            return 1
        fi
        label="$from"
    else
        source="$HOME/Library/Application Support/Claude"
        label="$CLAUDE_DESKTOP_STANDARD_LABEL"
    fi

    local -a items dest_items
    items=(${(f)"$(_claude_acc_desktop_runtime_items "$source")"})
    dest_items=(${(f)"$(_claude_acc_desktop_runtime_items "$profile")"})
    if (( ${#items} == 0 )); then
        _msg desktop_runtime_no_source "$label"
        return 1
    fi
    if (( ${#dest_items} > 0 )) && [[ "$force" != true ]]; then
        _msg desktop_runtime_keep
        return 1
    fi
    # clonefile can't cross filesystems, and cp -c silently falls back to a
    # real copy there — 10 GB spent to save 10 GB.
    local src_dev dst_dev probe="$profile"
    while [[ ! -e "$probe" && -n "$probe" ]]; do probe="${probe:h}"; done
    src_dev=$(stat -f '%d' "$source" 2>/dev/null)
    dst_dev=$(stat -f '%d' "$probe" 2>/dev/null)
    if [[ -z "$src_dev" || -z "$dst_dev" || "$src_dev" != "$dst_dev" ]]; then
        _msg desktop_runtime_would_copy
        return 1
    fi

    local rel dest n=0
    for rel in "${items[@]}"; do
        dest="$profile/$rel"
        mkdir -p "${dest:h}" || return 1
        # cp -R copies *into* an existing directory, which on a re-clone
        # would nest one version inside another.
        [[ -d "$dest" ]] && rm -rf "$dest"
        if ! /bin/cp -cR "$source/$rel" "$dest" 2>/dev/null; then
            rm -rf "$dest"
            _msg desktop_runtime_failed
            return 1
        fi
        (( n++ ))
    done
    _msg desktop_runtime_cloned "$n" "$label"
    _msg desktop_runtime_unverified
}

_claude_acc_desktop() {
    local action="$1"
    shift 2>/dev/null

    case "$action" in
        add)          _claude_acc_desktop_add "$@" ;;
        usage)        _msg desktop_usage_rust; return 1 ;;
        clone-config) _claude_acc_desktop_clone_config "$@" ;;
        clone-runtime) _claude_acc_desktop_clone_runtime "$@" ;;
        list)         _claude_acc_desktop_list ;;
        run)          _claude_acc_desktop_run "$@" ;;
        remove)       _claude_acc_desktop_remove "$@" ;;
        *)            _msg desktop_usage; return 1 ;;
    esac
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

# Echoes "<email>\t<uuid>\t<org>\t<plan>" or empty on failure. `plan` is a
# friendly tier label ("Max 20x" / "Pro" / "") derived from the boolean flags
# plus the multiplier in organization.rate_limit_tier — mirrors derive_plan()
# in the Rust src/identity.rs.
_claude_acc_identity() {
    local token="$1"
    [[ -z "$token" ]] && return 1
    curl -sf --max-time 5 \
        -H "Authorization: Bearer $token" \
        -H "anthropic-beta: oauth-2025-04-20" \
        https://api.anthropic.com/api/oauth/profile 2>/dev/null \
        | jq -r '
            (.account.has_claude_max // false) as $max
            | (.account.has_claude_pro // false) as $pro
            | (.organization.rate_limit_tier // "") as $tier
            | (($tier | [scan("[0-9]+x")][0]) // "") as $mult
            | (if $max then ("Max" + (if $mult != "" then " " + $mult else "" end))
               elif $pro then "Pro" else "" end) as $plan
            | "\(.account.email // "<unknown>")\t\(.account.uuid // "<unknown>")\t\(.organization.name // "")\t\($plan)"
          ' 2>/dev/null
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

# Cache for the unmanaged ~/.claude/ ("standard / default") config dir.
# Lives in our switch dir, never inside ~/.claude/ itself.
_claude_acc_default_cache_path() {
    echo "$CLAUDE_SWITCH_DIR/default.account-info.json"
}

_claude_acc_default_token_dir() {
    echo "$HOME/.claude"
}

_claude_acc_write_cache() {
    local cache="$1" email="$2" uuid="$3" org="$4" token="$5" plan="$6"
    local now hash
    now=$(date +%s)
    hash=$(_claude_acc_token_hash "$token")
    jq -n \
        --arg email "$email" \
        --arg uuid "$uuid" \
        --arg org "$org" \
        --argjson fetched_at "$now" \
        --arg token_hash "$hash" \
        --arg plan "$plan" \
        '{email:$email, uuid:$uuid, org:$org, fetched_at:$fetched_at, token_hash:$token_hash, plan:$plan}' \
        > "$cache" 2>/dev/null
}

_claude_acc_cache_plan() {
    local acc_dir="$CLAUDE_SWITCH_ACCOUNTS_DIR/$1"
    jq -r '.plan // empty' "$(_claude_acc_cache_path "$acc_dir")" 2>/dev/null
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

_claude_acc_default_email() {
    jq -r '.email // empty' "$(_claude_acc_default_cache_path)" 2>/dev/null
}

_claude_acc_default_plan() {
    jq -r '.plan // empty' "$(_claude_acc_default_cache_path)" 2>/dev/null
}

_claude_acc_default_drift_marker() {
    local cached current
    cached=$(jq -r '.token_hash // empty' "$(_claude_acc_default_cache_path)" 2>/dev/null)
    [[ -z "$cached" || "$cached" == "null" ]] && return 0
    current=$(_claude_acc_token_hash "$(_claude_acc_token "$(_claude_acc_default_token_dir)")")
    [[ -z "$current" ]] && return 0
    [[ "$cached" != "$current" ]] && printf ' *'
}

# Suffix for the unmanaged ~/.claude/ row in `list`. Empty if no cache or
# no email; otherwise "  email  Max 20x  3d ago" with optional ` *` drift marker.
_claude_acc_default_suffix() {
    local cache email fetched now secs when drift plan plan_seg
    cache=$(_claude_acc_default_cache_path)
    [[ ! -f "$cache" ]] && return 0
    email=$(jq -r '.email // empty' "$cache" 2>/dev/null)
    [[ -z "$email" ]] && return 0
    fetched=$(jq -r '.fetched_at // empty' "$cache" 2>/dev/null)
    when=""
    if [[ -n "$fetched" && "$fetched" != "null" ]]; then
        now=$(date +%s)
        if (( fetched <= now )); then
            secs=$(( now - fetched ))
            when=$(_claude_acc_relative_time "$secs")
        fi
    fi
    plan=$(_claude_acc_default_plan)
    plan_seg=""
    [[ -n "$plan" ]] && plan_seg="  $plan"
    drift=$(_claude_acc_default_drift_marker)
    if [[ -n "$when" ]]; then
        printf '  %s%s  %s%s' "$email" "$plan_seg" "$when" "$drift"
    else
        printf '  %s%s%s' "$email" "$plan_seg" "$drift"
    fi
}

# Rendered "  email  Max 20x  3d ago" / "… *" / "" suffix for list/status.
_claude_acc_identity_suffix() {
    local name="$1" email when secs drift plan plan_seg
    email=$(_claude_acc_cache_email "$name")
    [[ -z "$email" ]] && return 0
    secs=$(_claude_acc_cache_age "$name")
    if [[ -n "$secs" ]]; then
        when=$(_claude_acc_relative_time "$secs")
    fi
    plan=$(_claude_acc_cache_plan "$name")
    plan_seg=""
    [[ -n "$plan" ]] && plan_seg="  $plan"
    drift=$(_claude_acc_cache_drift_marker "$name")
    if [[ -n "$when" ]]; then
        printf '  %s%s  %s%s' "$email" "$plan_seg" "$when" "$drift"
    else
        printf '  %s%s%s' "$email" "$plan_seg" "$drift"
    fi
}

# --- Usage (5-hour / 7-day rate-limit windows) ---
# Queries /api/oauth/usage per account and renders a bar + reset countdown for
# the five_hour and seven_day windows. Live-only (no cache) — usage is volatile.

# Format positive seconds as a short forward duration: "45m" / "2h 14m" /
# "5d 17h" / "<1m". Locale follows _claude_acc_lang. Mirror of the Rust
# forward_duration() in src/i18n.rs.
_claude_acc_forward_duration() {
    local secs="$1" lang n h m d
    lang=$(_claude_acc_lang)
    if (( secs < 60 )); then
        [[ "$lang" == "ru" ]] && echo "<1м" || echo "<1m"
        return
    fi
    if (( secs < 3600 )); then
        n=$(( secs / 60 ))
        [[ "$lang" == "ru" ]] && echo "${n}м" || echo "${n}m"
        return
    fi
    if (( secs < 86400 )); then
        h=$(( secs / 3600 )); m=$(( (secs % 3600) / 60 ))
        [[ "$lang" == "ru" ]] && echo "${h}ч ${m}м" || echo "${h}h ${m}m"
        return
    fi
    d=$(( secs / 86400 )); h=$(( (secs % 86400) / 3600 ))
    [[ "$lang" == "ru" ]] && echo "${d}д ${h}ч" || echo "${d}d ${h}h"
}

# Render a 20-cell "[████░░░░…]" bar for an integer 0–100 percentage.
_claude_acc_usage_bar() {
    local pct="$1" width=20 filled empty bar="" i
    (( pct < 0 )) && pct=0
    (( pct > 100 )) && pct=100
    filled=$(( (pct * width + 50) / 100 ))   # round(pct/100 * width)
    (( filled > width )) && filled=width
    empty=$(( width - filled ))
    for (( i = 0; i < filled; i++ )); do bar+="█"; done
    for (( i = 0; i < empty; i++ )); do bar+="░"; done
    printf '[%s]' "$bar"
}

# Fetch + parse usage for a token. Emits one TAB-separated line per non-null
# window: "<label>\t<pct>\t<remaining_secs>". remaining_secs is empty when the
# window has no scheduled reset. Empty output (rc 1) means offline / bad token.
_claude_acc_usage_fetch() {
    local token="$1" json now
    [[ -z "$token" ]] && return 1
    json=$(curl -sf --max-time 5 \
        -H "Authorization: Bearer $token" \
        -H "anthropic-beta: oauth-2025-04-20" \
        https://api.anthropic.com/api/oauth/usage 2>/dev/null) || return 1
    [[ -z "$json" ]] && return 1
    now=$(date +%s)
    printf '%s' "$json" | jq -r --argjson now "$now" '
        def remain(w):
            if (w == null) or (w.resets_at == null) then ""
            else ((w.resets_at
                   | sub("\\.[0-9]+"; "")
                   | sub("Z$"; "")
                   | sub("[+-][0-9]{2}:[0-9]{2}$"; "")
                   | strptime("%Y-%m-%dT%H:%M:%S") | mktime) - $now)
            end;
        def util(w): (w.utilization // 0 | round);
        def line(name; w):
            if w == null then empty
            else "\(name)\t\(util(w))\t\(remain(w))" end;
        line("5h"; .five_hour), line("7d"; .seven_day)
    ' 2>/dev/null
}

# Print the per-window lines for one token dir (managed account or ~/.claude/).
_claude_acc_usage_render() {
    local token_dir="$1" name="$2" token out key pct remain reset bar
    token=$(_claude_acc_token "$token_dir")
    if [[ -z "$token" ]]; then
        printf "      %s\n" "$(_msg doctor_no_token "$name")"
        return
    fi
    out=$(_claude_acc_usage_fetch "$token")
    if [[ -z "$out" ]]; then
        printf "      %s\n" "$(_msg doctor_offline)"
        return
    fi
    while IFS=$'\t' read -r key pct remain; do
        [[ -z "$key" ]] && continue
        bar=$(_claude_acc_usage_bar "$pct")
        if [[ -z "$remain" ]]; then
            reset=""
        elif (( remain > 0 )); then
            reset=$(_msg usage_resets_in "$(_claude_acc_forward_duration "$remain")")
        else
            reset=$(_msg usage_available_now)
        fi
        printf "      %s  %s  %3d%%  %s\n" "$key" "$bar" "$pct" "$reset"
    done <<< "$out"
}

_claude_acc_usage() {
    local dep
    for dep in security curl jq shasum; do
        if ! command -v "$dep" >/dev/null 2>&1; then
            _msg usage_missing_dep "$dep"
            return 1
        fi
    done

    local default_acc
    default_acc=$(_claude_default_account)
    local accounts=("$CLAUDE_SWITCH_ACCOUNTS_DIR"/*(N:t))
    local standard_dir standard_present=0
    standard_dir=$(_claude_acc_default_token_dir)
    [[ -n "$(_claude_acc_token "$standard_dir")" ]] && standard_present=1

    if (( ${#accounts} == 0 && standard_present == 0 )); then
        _msg list_empty
        return 0
    fi

    _msg usage_header

    local acc marker email plan plan_seg
    for acc in "${accounts[@]}"; do
        marker=" "
        [[ "$acc" == "$default_acc" ]] && marker="★"
        email=$(_claude_acc_cache_email "$acc")
        plan=$(_claude_acc_cache_plan "$acc")
        plan_seg=""; [[ -n "$plan" ]] && plan_seg="  $plan"
        if [[ -n "$email" ]]; then
            printf "  %s %s  <%s>%s\n" "$marker" "$acc" "$email" "$plan_seg"
        else
            printf "  %s %s\n" "$marker" "$acc"
        fi
        _claude_acc_usage_render "$CLAUDE_SWITCH_ACCOUNTS_DIR/$acc" "$acc"
    done

    if (( standard_present )); then
        email=$(_claude_acc_default_email)
        plan=$(_claude_acc_default_plan)
        plan_seg=""; [[ -n "$plan" ]] && plan_seg="  $plan"
        if [[ -n "$email" ]]; then
            printf "    ~/.claude/  <%s>%s  %s\n" "$email" "$plan_seg" "$(_msg list_standard)"
        else
            printf "    ~/.claude/  %s\n" "$(_msg list_standard)"
        fi
        _claude_acc_usage_render "$standard_dir" "~/.claude/"
    fi
}

# Update the sourced claude-switch.sh in place from GitHub (master). The Rust
# CLI updates a versioned binary; the shell distribution is a single sourced
# file, so "update" just re-fetches the latest script and asks you to re-source.
_claude_acc_update() {
    command -v curl >/dev/null 2>&1 || { _msg update_missing_dep curl; return 1; }

    local script="$CLAUDE_SWITCH_SCRIPT"
    if [[ -z "$script" || ! -f "$script" ]]; then
        _msg update_script_unknown
        return 1
    fi

    local raw="https://raw.githubusercontent.com/Nemo-Illusionist/claude-code-account-switcher/master/claude-switch.sh"
    local tmp="${script}.new"

    _msg update_fetching
    if ! curl -fsSL --max-time 60 -H "User-Agent: claude-acc" -o "$tmp" "$raw"; then
        rm -f "$tmp"
        _msg update_download_failed
        return 1
    fi
    # Sanity-check the payload before overwriting the live script.
    if ! grep -q "Claude Code Account Switcher" "$tmp" 2>/dev/null; then
        rm -f "$tmp"
        _msg update_download_failed
        return 1
    fi

    mv "$tmp" "$script"
    _msg update_done "$script" "$script"
}

# Re-key the macOS Keychain OAuth entry from one config-dir path to another.
# Claude Code keys the token by the absolute config-dir path, so a copied/moved
# dir would otherwise lose its token. No-op (returns non-zero) off macOS or when
# there's no source entry — the plaintext .credentials.json fallback covers it.
_claude_acc_rekey_keychain() {
    local from="$1" to="$2" user fromhash tohash blob
    [[ "$(uname -s)" == "Darwin" ]] || return 1
    command -v security >/dev/null 2>&1 || return 1
    command -v shasum >/dev/null 2>&1 || return 1
    user=$(id -un)
    fromhash=$(printf '%s' "$from" | shasum -a 256 | cut -c1-8)
    tohash=$(printf '%s' "$to" | shasum -a 256 | cut -c1-8)
    blob=$(security find-generic-password -s "Claude Code-credentials-${fromhash}" -a "$user" -w 2>/dev/null) || return 1
    [[ -z "$blob" ]] && return 1
    security add-generic-password -U -s "Claude Code-credentials-${tohash}" -a "$user" -w "$blob" 2>/dev/null
}

# Adopt an existing Claude config dir as a managed account without re-login.
_claude_acc_import() {
    local move=0 a
    local -a pos
    for a in "$@"; do
        if [[ "$a" == "--move" ]]; then move=1; else pos+=("$a"); fi
    done
    local name="${pos[1]}" source="${pos[2]}"
    if [[ -z "$name" || -z "$source" ]]; then _msg import_usage; return 1; fi
    if [[ "$name" == "default" ]]; then _msg reserved_name "$name"; return 1; fi
    _claude_validate_name "$name" || return 1

    # `:a` makes the path absolute WITHOUT resolving symlinks — Claude Code
    # keyed the Keychain token by the config-dir path as given (e.g. it never
    # resolves /tmp -> /private/tmp), so `:A` would compute a non-matching hash.
    local src="${source:a}"
    if [[ ! -d "$src" ]]; then _msg import_source_not_dir "$source"; return 1; fi
    local target="$CLAUDE_SWITCH_ACCOUNTS_DIR/$name"
    if [[ -e "$target" ]]; then _msg add_exists "$name"; return 1; fi
    if [[ "$src" == "$CLAUDE_SWITCH_ACCOUNTS_DIR"/* ]]; then _msg import_source_managed; return 1; fi

    if (( move )); then
        mv "$src" "$target" 2>/dev/null || { _msg import_failed "mv"; return 1; }
    else
        cp -R "$src" "$target" 2>/dev/null || { rm -rf "$target"; _msg import_failed "cp"; return 1; }
    fi

    local rekeyed=0
    _claude_acc_rekey_keychain "$src" "$target" && rekeyed=1

    _msg import_done "$name" "$target"
    (( rekeyed )) && _msg import_rekeyed

    # Audit: confirm the imported dir resolves, and seed the doctor cache.
    local token identity email uuid org plan
    token=$(_claude_acc_token "$target")
    if [[ -z "$token" ]]; then _msg doctor_no_token "$name"; return 0; fi
    identity=$(_claude_acc_identity "$token")
    if [[ -z "$identity" ]]; then _msg doctor_offline; return 0; fi
    IFS=$'\t' read -r email uuid org plan <<< "$identity"
    _claude_acc_write_cache "$(_claude_acc_cache_path "$target")" "$email" "$uuid" "$org" "$token" "$plan"
    _msg import_verified "$email"
}

_claude_acc_doctor() {
    local json=0
    if [[ "$1" == "--json" ]]; then
        json=1
        shift
    fi

    local dep
    for dep in security curl jq shasum; do
        if ! command -v "$dep" >/dev/null 2>&1; then
            _msg doctor_missing_dep "$dep"
            return 1
        fi
    done

    local accounts=("$CLAUDE_SWITCH_ACCOUNTS_DIR"/*(N:t))
    local standard_label="~/.claude/"
    local standard_dir
    standard_dir=$(_claude_acc_default_token_dir)
    local standard_present=0
    [[ -n "$(_claude_acc_token "$standard_dir")" ]] && standard_present=1

    if (( json )); then
        _claude_acc_doctor_json "${accounts[@]}" "$standard_present"
        return $?
    fi

    if (( ${#accounts} == 0 && standard_present == 0 )); then
        _msg list_empty
        return 0
    fi

    local total=$(( ${#accounts} + standard_present ))
    _msg doctor_header "$total"

    # Compute label width for aligned output.
    local width=0 acc
    for acc in "${accounts[@]}"; do
        (( ${#acc} > width )) && width=${#acc}
    done
    if (( standard_present )); then
        (( ${#standard_label} > width )) && width=${#standard_label}
    fi

    # Pass 1 — audit everything and collect rows. We print only after building
    # the UUID cross-reference, so the shared-identity note can list siblings.
    local -a r_label r_kind r_status r_email r_uuid r_plan
    local -A uuid_names
    # `acc` is already declared above (width loop); re-`local`-ing it here would
    # make zsh's typeset echo "acc=<value>" to stdout.
    local acc_dir token identity email uuid org plan
    for acc in "${accounts[@]}"; do
        acc_dir="$CLAUDE_SWITCH_ACCOUNTS_DIR/$acc"
        token=$(_claude_acc_token "$acc_dir")
        if [[ -z "$token" ]]; then
            r_label+=("$acc"); r_kind+=("managed"); r_status+=("no_token")
            r_email+=(""); r_uuid+=(""); r_plan+=("")
            continue
        fi
        identity=$(_claude_acc_identity "$token")
        if [[ -z "$identity" ]]; then
            r_label+=("$acc"); r_kind+=("managed"); r_status+=("offline")
            r_email+=(""); r_uuid+=(""); r_plan+=("")
            continue
        fi
        IFS=$'\t' read -r email uuid org plan <<< "$identity"
        _claude_acc_write_cache "$(_claude_acc_cache_path "$acc_dir")" \
            "$email" "$uuid" "$org" "$token" "$plan"
        r_label+=("$acc"); r_kind+=("managed"); r_status+=("ok")
        r_email+=("$email"); r_uuid+=("$uuid"); r_plan+=("$plan")
        if [[ -n "$uuid" && "$uuid" != "<unknown>" ]]; then
            [[ -n "${uuid_names[$uuid]}" ]] && uuid_names[$uuid]+=$'\n'
            uuid_names[$uuid]+="$acc"
        fi
    done

    if (( standard_present )); then
        token=$(_claude_acc_token "$standard_dir")
        identity=$(_claude_acc_identity "$token")
        if [[ -z "$identity" ]]; then
            r_label+=("$standard_label"); r_kind+=("standard"); r_status+=("offline")
            r_email+=(""); r_uuid+=(""); r_plan+=("")
        else
            IFS=$'\t' read -r email uuid org plan <<< "$identity"
            _claude_acc_write_cache "$(_claude_acc_default_cache_path)" \
                "$email" "$uuid" "$org" "$token" "$plan"
            r_label+=("$standard_label"); r_kind+=("standard"); r_status+=("ok")
            r_email+=("$email"); r_uuid+=("$uuid"); r_plan+=("$plan")
            if [[ -n "$uuid" && "$uuid" != "<unknown>" ]]; then
                [[ -n "${uuid_names[$uuid]}" ]] && uuid_names[$uuid]+=$'\n'
                uuid_names[$uuid]+="$standard_label"
            fi
        fi
    fi

    # Pass 2 — render, annotating accounts that share a UUID. ('status' is a
    # read-only special parameter in zsh, so the row state lives in `st`.)
    local healthy=0 i label kind st plan_seg others nm
    local -a sib keep
    for (( i = 1; i <= ${#r_label}; i++ )); do
        label="${r_label[$i]}"; kind="${r_kind[$i]}"; st="${r_status[$i]}"
        email="${r_email[$i]}"; uuid="${r_uuid[$i]}"; plan="${r_plan[$i]}"
        case "$st" in
            no_token)
                printf "  ? %-${width}s  %s\n" "$label" "$(_msg doctor_no_token "$label")"
                ;;
            offline)
                printf "  ? %-${width}s  %s\n" "$label" "$(_msg doctor_offline)"
                ;;
            ok)
                (( healthy++ ))
                plan_seg=""; [[ -n "$plan" ]] && plan_seg="  $plan"
                others=""
                if [[ -n "$uuid" && "$uuid" != "<unknown>" ]]; then
                    sib=("${(@f)uuid_names[$uuid]}"); keep=()
                    for nm in "${sib[@]}"; do
                        [[ "$nm" != "$label" ]] && keep+=("$nm")
                    done
                    (( ${#keep} > 0 )) && others="  $(_msg doctor_shared_identity "${(j:, :)keep}")"
                fi
                if [[ "$kind" == "standard" ]]; then
                    printf "  ✓ %-${width}s  %s%s  uuid=%s  (%s)%s\n" \
                        "$label" "$email" "$plan_seg" "$uuid" "$(_msg list_standard)" "$others"
                else
                    printf "  ✓ %-${width}s  %s%s  uuid=%s%s\n" \
                        "$label" "$email" "$plan_seg" "$uuid" "$others"
                fi
                ;;
        esac
    done

    echo ""
    if (( healthy == total )); then
        _msg doctor_all_ok
        return 0
    else
        _msg doctor_partial "$healthy" "$total"
        return 1
    fi
}

# JSON form of `doctor`. Last positional arg is the standard_present flag (0/1);
# preceding args are managed-account names. Schema matches the Rust binary —
# see src/commands/doctor.rs.
_claude_acc_doctor_json() {
    local args=("$@")
    local n=${#args[@]}
    local standard_present="${args[$n]}"
    local accounts=("${(@)args[1,$((n-1))]}")
    local default_acc
    default_acc=$(_claude_default_account)
    local any_problem=0

    local entries="[]"
    local acc acc_dir token identity audit_status email uuid org plan is_default entry
    for acc in "${accounts[@]}"; do
        acc_dir="$CLAUDE_SWITCH_ACCOUNTS_DIR/$acc"
        token=$(_claude_acc_token "$acc_dir")
        if [[ -z "$token" ]]; then
            audit_status="no_token"; email=""; uuid=""; plan=""
        else
            identity=$(_claude_acc_identity "$token")
            if [[ -z "$identity" ]]; then
                audit_status="offline"; email=""; uuid=""; plan=""
                any_problem=1
            else
                audit_status="ok"
                IFS=$'\t' read -r email uuid org plan <<< "$identity"
                _claude_acc_write_cache "$(_claude_acc_cache_path "$acc_dir")" \
                    "$email" "$uuid" "$org" "$token" "$plan"
            fi
        fi
        is_default=false
        [[ "$acc" == "$default_acc" ]] && is_default=true
        entry=$(jq -n \
            --arg name "$acc" --arg status "$audit_status" \
            --arg email "$email" --arg uuid "$uuid" --arg plan "$plan" \
            --argjson is_default "$is_default" \
            '{name:$name, status:$status,
              email:(if $email == "" then null else $email end),
              uuid:(if $uuid == "" then null else $uuid end),
              plan:(if $plan == "" then null else $plan end),
              default:$is_default}')
        entries=$(jq --argjson e "$entry" '. + [$e]' <<< "$entries")
    done

    local standard="null"
    if (( standard_present )); then
        token=$(_claude_acc_token "$standard_dir" 2>/dev/null) || \
            token=$(_claude_acc_token "$(_claude_acc_default_token_dir)")
        identity=$(_claude_acc_identity "$token")
        if [[ -z "$identity" ]]; then
            audit_status="offline"; email=""; uuid=""; plan=""
            any_problem=1
        else
            audit_status="ok"
            IFS=$'\t' read -r email uuid org plan <<< "$identity"
            _claude_acc_write_cache "$(_claude_acc_default_cache_path)" \
                "$email" "$uuid" "$org" "$token" "$plan"
        fi
        standard=$(jq -n \
            --arg name "~/.claude/" --arg status "$audit_status" \
            --arg email "$email" --arg uuid "$uuid" --arg plan "$plan" \
            '{name:$name, status:$status,
              email:(if $email == "" then null else $email end),
              uuid:(if $uuid == "" then null else $uuid end),
              plan:(if $plan == "" then null else $plan end)}')
    fi

    jq -n --argjson accounts "$entries" --argjson standard "$standard" \
        '{accounts:$accounts, standard:$standard}'
    return $any_problem
}

# `whoami` — emit the most-identifying string for the active account.
# Order: cached email > account name > literal "default" for standard.
_claude_acc_whoami() {
    local linked_dir account email
    linked_dir=$(_claude_find_linked_dir)
    if [[ -n "$linked_dir" ]]; then
        account=$(_claude_dir_account "$linked_dir")
    fi
    [[ -z "$account" ]] && account=$(_claude_default_account)

    if [[ -n "$account" && "$account" != "default" ]]; then
        email=$(_claude_acc_cache_email "$account")
        echo "${email:-$account}"
        return 0
    fi

    # Standard ~/.claude/.
    email=$(_claude_acc_default_email)
    echo "${email:-default}"
}

# =============================================================
# Single entry point
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
        usage)   _claude_acc_usage "$@" ;;
        update)  _claude_acc_update "$@" ;;
        run)     _claude_acc_run "$@" ;;
        doctor)  _claude_acc_doctor "$@" ;;
        whoami)  _claude_acc_whoami ;;
        clone-settings) _claude_acc_clone_settings "$@" ;;
        import)  _claude_acc_import "$@" ;;
        desktop) _claude_acc_desktop "$@" ;;
        help)    _claude_acc_help ;;
        *)       _claude_acc_help ;;
    esac
}

# =============================================================
# Tab completion (zsh)
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
        "usage:$(_msg help_usage)"
        "update:$(_msg help_update)"
        "run:$(_msg help_run)"
        "doctor:$(_msg help_doctor)"
        "whoami:$(_msg help_whoami)"
        "clone-settings:$(_msg help_clone_settings)"
        "import:$(_msg help_import)"
        "desktop:$(_msg help_desktop)"
        "help:$(_msg help_help)"
    )

    if (( CURRENT == 2 )); then
        _describe 'command' subcmds
    elif (( CURRENT == 3 )); then
        case "${words[2]}" in
            remove|clone-settings)
                accounts=("$CLAUDE_SWITCH_ACCOUNTS_DIR"/*(N:t))
                _describe 'account' accounts
                ;;
            default|link|login|run)
                accounts=("default" "$CLAUDE_SWITCH_ACCOUNTS_DIR"/*(N:t))
                _describe 'account' accounts
                ;;
            desktop)
                local -a desktop_actions
                desktop_actions=(add clone-config clone-runtime list run remove)
                _describe 'action' desktop_actions
                ;;
        esac
    elif (( CURRENT == 4 )) && [[ "${words[2]}" == "import" ]]; then
        # import <name> <path> — complete a directory for the source path.
        _path_files -/
    elif (( CURRENT == 4 )) && [[ "${words[2]}" == "desktop" ]]; then
        # desktop add <name> is a new name; run/remove take an existing one.
        case "${words[3]}" in
            run|remove|clone-config|clone-runtime)
                local -a profiles
                profiles=("$CLAUDE_SWITCH_DESKTOP_DIR"/*(N/:t))
                _describe 'profile' profiles
                ;;
        esac
    fi
}

compdef _claude_acc_completion claude-acc

# =============================================================
# PATH: put the wrapper ahead of the real claude
# =============================================================
# Only prepend if not already in PATH, to avoid duplication on re-source.
if [[ ":$PATH:" != *":$CLAUDE_SWITCH_BIN:"* ]]; then
    export PATH="$CLAUDE_SWITCH_BIN:$PATH"
fi
