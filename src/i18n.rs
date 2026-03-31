#[derive(Clone, Copy, PartialEq)]
pub enum Lang {
    En,
    Ru,
}

impl Lang {
    pub fn detect() -> Self {
        if let Ok(v) = std::env::var("CLAUDE_ACC_LANG") {
            return match v.as_str() {
                "ru" => Lang::Ru,
                _ => Lang::En,
            };
        }
        if let Ok(v) = std::env::var("LANG") {
            if v.starts_with("ru") {
                return Lang::Ru;
            }
        }
        Lang::En
    }
}

pub struct I18n {
    pub lang: Lang,
}

impl I18n {
    pub fn new() -> Self {
        Self { lang: Lang::detect() }
    }

    pub fn msg(&self, key: Msg) -> String {
        match (key, self.lang) {
            // help
            (Msg::HelpTitle, _) => s("Claude Code Account Switcher"),
            (Msg::HelpCommands, Lang::En) => s("Commands:"),
            (Msg::HelpCommands, Lang::Ru) => s("Команды:"),

            // list
            (Msg::ListEmpty, Lang::En) => s("No accounts. Add one: claude-acc add <name>"),
            (Msg::ListEmpty, Lang::Ru) => s("Нет аккаунтов. Добавьте: claude-acc add <name>"),
            (Msg::ListHeader, Lang::En) => s("Claude Code accounts:"),
            (Msg::ListHeader, Lang::Ru) => s("Аккаунты Claude Code:"),
            (Msg::ListDefault, Lang::En) => s("(default)"),
            (Msg::ListDefault, Lang::Ru) => s("(по умолчанию)"),

            // add
            (Msg::AddExists(ref n), Lang::En) => format!("Account '{}' already exists.", n),
            (Msg::AddExists(ref n), Lang::Ru) => format!("Аккаунт '{}' уже существует.", n),
            (Msg::AddCreated(ref n), Lang::En) => format!("Account '{}' created. Starting login...", n),
            (Msg::AddCreated(ref n), Lang::Ru) => format!("Аккаунт '{}' создан. Запускаю логин...", n),
            (Msg::AddDone, Lang::En) => s("Done. Use:"),
            (Msg::AddDone, Lang::Ru) => s("Готово. Используйте:"),
            (Msg::AddHintDefault(ref n), Lang::En) => format!("  claude-acc default {}   — set as default", n),
            (Msg::AddHintDefault(ref n), Lang::Ru) => format!("  claude-acc default {}   — сделать дефолтным", n),
            (Msg::AddHintLink(ref n), Lang::En) => format!("  claude-acc link {}      — link to current directory", n),
            (Msg::AddHintLink(ref n), Lang::Ru) => format!("  claude-acc link {}      — привязать к текущей директории", n),

            // login
            (Msg::LoginNotFound(ref n), Lang::En) => format!("Account '{}' not found.", n),
            (Msg::LoginNotFound(ref n), Lang::Ru) => format!("Аккаунт '{}' не найден.", n),
            (Msg::LoginStart(ref n), Lang::En) => format!("Logging in to '{}'...", n),
            (Msg::LoginStart(ref n), Lang::Ru) => format!("Вхожу в '{}'...", n),
            (Msg::LoginDone, Lang::En) => s("Done."),
            (Msg::LoginDone, Lang::Ru) => s("Готово."),

            // remove
            (Msg::RemoveNotFound(ref n), Lang::En) => format!("Account '{}' not found.", n),
            (Msg::RemoveNotFound(ref n), Lang::Ru) => format!("Аккаунт '{}' не найден.", n),
            (Msg::RemoveConfirm(ref n), Lang::En) => format!("Remove account '{}'? [y/N] ", n),
            (Msg::RemoveConfirm(ref n), Lang::Ru) => format!("Удалить аккаунт '{}'? [y/N] ", n),
            (Msg::RemoveCancelled, Lang::En) => s("Cancelled."),
            (Msg::RemoveCancelled, Lang::Ru) => s("Отменено."),
            (Msg::RemoveDeleted(ref n), Lang::En) => format!("Account '{}' deleted.", n),
            (Msg::RemoveDeleted(ref n), Lang::Ru) => format!("Аккаунт '{}' удалён.", n),

            // default
            (Msg::DefaultCurrent(ref n), Lang::En) => format!("Default: {}", n),
            (Msg::DefaultCurrent(ref n), Lang::Ru) => format!("По умолчанию: {}", n),
            (Msg::DefaultStandard, Lang::En) => s("Default: ~/.claude/"),
            (Msg::DefaultStandard, Lang::Ru) => s("По умолчанию: ~/.claude/"),
            (Msg::DefaultNotFound(ref n), Lang::En) => format!("Account '{}' not found. Available:", n),
            (Msg::DefaultNotFound(ref n), Lang::Ru) => format!("Аккаунт '{}' не найден. Доступные:", n),
            (Msg::DefaultSet(ref n), Lang::En) => format!("Default account: {}", n),
            (Msg::DefaultSet(ref n), Lang::Ru) => format!("Аккаунт по умолчанию: {}", n),
            (Msg::ResetDone, Lang::En) => s("Reset to ~/.claude/"),
            (Msg::ResetDone, Lang::Ru) => s("Сброшено на ~/.claude/"),

            // link
            (Msg::LinkNotFound(ref n), Lang::En) => format!("Account '{}' not found. Available:", n),
            (Msg::LinkNotFound(ref n), Lang::Ru) => format!("Аккаунт '{}' не найден. Доступные:", n),
            (Msg::LinkDone(ref dir, ref n), Lang::En) => format!("{} → account '{}'", dir, n),
            (Msg::LinkDone(ref dir, ref n), Lang::Ru) => format!("{} → аккаунт '{}'", dir, n),
            (Msg::LinkDoneDefault(ref dir), Lang::En) => format!("{} → ~/.claude/ (default)", dir),
            (Msg::LinkDoneDefault(ref dir), Lang::Ru) => format!("{} → ~/.claude/ (default)", dir),

            // unlink
            (Msg::UnlinkNone, Lang::En) => s("No link for the current directory."),
            (Msg::UnlinkNone, Lang::Ru) => s("Нет привязки для текущей директории."),
            (Msg::UnlinkDone(ref dir), Lang::En) => format!("Unlinked {}. Default account will be used.", dir),
            (Msg::UnlinkDone(ref dir), Lang::Ru) => format!("Привязка убрана для {}. Будет использован дефолтный аккаунт.", dir),

            // status
            (Msg::StatusActive(ref n, ref info), Lang::En) => format!("Active account: {} {}", n, info),
            (Msg::StatusActive(ref n, ref info), Lang::Ru) => format!("Активный аккаунт: {} {}", n, info),
            (Msg::StatusLinked(ref dir), Lang::En) => format!("(linked to {})", dir),
            (Msg::StatusLinked(ref dir), Lang::Ru) => format!("(привязан к {})", dir),
            (Msg::StatusDefault, Lang::En) => s("(default)"),
            (Msg::StatusDefault, Lang::Ru) => s("(по умолчанию)"),
            (Msg::StatusStandard, Lang::En) => s("Active account: ~/.claude/ (standard)"),
            (Msg::StatusStandard, Lang::Ru) => s("Активный аккаунт: ~/.claude/ (стандартный)"),

            // links
            (Msg::LinksEmpty, Lang::En) => s("No links. Use: claude-acc link <name>"),
            (Msg::LinksEmpty, Lang::Ru) => s("Нет привязок. Используйте: claude-acc link <name>"),
            (Msg::LinksHeader, Lang::En) => s("Links:"),
            (Msg::LinksHeader, Lang::Ru) => s("Привязки:"),
            (Msg::LinksActive, Lang::En) => s("← active"),
            (Msg::LinksActive, Lang::Ru) => s("← активна"),

            // reserved
            (Msg::ReservedName(ref n), Lang::En) => format!("'{}' is a reserved name.", n),
            (Msg::ReservedName(ref n), Lang::Ru) => format!("'{}' — зарезервированное имя.", n),

            // install
            (Msg::InstallUpToDate(ref v), Lang::En) => format!("Already up to date (v{}).", v),
            (Msg::InstallUpToDate(ref v), Lang::Ru) => format!("Уже актуальная версия (v{}).", v),
            (Msg::InstallUpdating(ref old, ref new), Lang::En) => format!("Updating v{} → v{}...", old, new),
            (Msg::InstallUpdating(ref old, ref new), Lang::Ru) => format!("Обновление v{} → v{}...", old, new),
            (Msg::InstallCopying(ref v), Lang::En) => format!("Installing v{}...", v),
            (Msg::InstallCopying(ref v), Lang::Ru) => format!("Установка v{}...", v),
            (Msg::InstallDone(ref p), Lang::En) => format!("Binary installed: {}", p),
            (Msg::InstallDone(ref p), Lang::Ru) => format!("Бинарник установлен: {}", p),
            (Msg::InstallShellAlready(ref f), Lang::En) => format!("Shell integration already in {}", f),
            (Msg::InstallShellAlready(ref f), Lang::Ru) => format!("Shell-интеграция уже в {}", f),
            (Msg::InstallShellUpdated(ref f), Lang::En) => format!("Shell integration updated in {}", f),
            (Msg::InstallShellUpdated(ref f), Lang::Ru) => format!("Shell-интеграция обновлена в {}", f),
            (Msg::InstallShellAdded(ref f), Lang::En) => format!("Shell integration added to {}", f),
            (Msg::InstallShellAdded(ref f), Lang::Ru) => format!("Shell-интеграция добавлена в {}", f),
            (Msg::InstallShellManual(ref line), Lang::En) => format!("Add this to your shell config:\n  {}", line),
            (Msg::InstallShellManual(ref line), Lang::Ru) => format!("Добавьте в конфиг шелла:\n  {}", line),
        }
    }

    pub fn print(&self, key: Msg) {
        println!("{}", self.msg(key));
    }
}

fn s(v: &str) -> String {
    v.to_string()
}

#[derive(Clone)]
#[allow(dead_code)]
pub enum Msg {
    HelpTitle,
    HelpCommands,
    ListEmpty,
    ListHeader,
    ListDefault,
    AddExists(String),
    AddCreated(String),
    AddDone,
    AddHintDefault(String),
    AddHintLink(String),
    LoginNotFound(String),
    LoginStart(String),
    LoginDone,
    RemoveNotFound(String),
    RemoveConfirm(String),
    RemoveCancelled,
    RemoveDeleted(String),
    DefaultCurrent(String),
    DefaultStandard,
    DefaultNotFound(String),
    DefaultSet(String),
    ResetDone,
    LinkNotFound(String),
    LinkDone(String, String),
    LinkDoneDefault(String),
    UnlinkNone,
    UnlinkDone(String),
    StatusActive(String, String),
    StatusLinked(String),
    StatusDefault,
    StatusStandard,
    LinksEmpty,
    LinksHeader,
    LinksActive,
    ReservedName(String),
    InstallUpToDate(String),
    InstallUpdating(String, String),
    InstallCopying(String),
    InstallDone(String),
    InstallShellAlready(String),
    InstallShellUpdated(String),
    InstallShellAdded(String),
    InstallShellManual(String),
}
