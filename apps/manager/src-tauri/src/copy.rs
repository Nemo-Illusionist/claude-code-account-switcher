//! The prose the shell itself shows (#166): the menu bar, the folder
//! picker's title, startup failures and the launcher's prompt in Terminal.
//! Not here: `productName` and the window's initial `title` in
//! `tauri.conf.json`, and anything macOS draws itself, such as the folder
//! picker's buttons. The webview's prose is `src/i18n.tsx`; this is the
//! native side of the same rule.
//!
//! `text` matches exhaustively over `(Text, Lang)`, as the CLI's `i18n.rs`
//! does, and has no catch-all: a language added to `Lang` fails to compile
//! until every text has an arm in it.
use claude_acc::gui::UsageSource;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    En,
}

/// The only language with a complete table today. How it is chosen once
/// there are two is the open question in the frontend's pull request (#172).
pub const LANG: Lang = Lang::En;

#[derive(Debug, Clone, Copy)]
pub enum Text<'a> {
    /// The startup error dialog's title.
    ProductName,
    TrayTooltip,
    /// Shown in place of the icon if the bundle has lost it.
    TrayTitle,
    OpenManager,
    ProjectBindings,
    Refresh,
    Quit,
    /// The 5-hour figure in an account's menu entry.
    /// Already rounded by the caller, so every language shows one figure.
    FiveHour(i64),
    Source(UsageSource),
    ChooseDirectory,
    NoHome,
    StartFailed(&'a str),
    IconMissing,
    /// Printed by the launcher before the CLI's exit status. Only the macOS
    /// launcher exists to print it.
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    LauncherExited,
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    LauncherPressReturn,
}

pub fn text(t: Text, lang: Lang) -> String {
    match (t, lang) {
        (Text::ProductName, Lang::En) => "Claude Accounts".into(),
        (Text::TrayTooltip, Lang::En) => "Claude Accounts — per-project accounts".into(),
        (Text::TrayTitle, Lang::En) => "Accounts".into(),
        (Text::OpenManager, Lang::En) => "Open account manager".into(),
        (Text::ProjectBindings, Lang::En) => "Project bindings…".into(),
        (Text::Refresh, Lang::En) => "Refresh identity & usage".into(),
        (Text::Quit, Lang::En) => "Quit".into(),
        (Text::FiveHour(percent), Lang::En) => format!("5h {percent}%"),
        (Text::Source(UsageSource::Live), Lang::En) => "live".into(),
        (Text::Source(UsageSource::Cached), Lang::En) => "cached".into(),
        (Text::Source(UsageSource::NoCredentials), Lang::En) => "no credentials".into(),
        (Text::Source(UsageSource::Unavailable), Lang::En) => "no reading".into(),
        (Text::ChooseDirectory, Lang::En) => "Choose project directory".into(),
        (Text::NoHome, Lang::En) => "Claude Accounts cannot find your home directory, so it has \
             no account configuration to read. Check that HOME is set."
            .into(),
        (Text::StartFailed(reason), Lang::En) => {
            format!("Claude Accounts could not start: {reason}")
        }
        (Text::IconMissing, Lang::En) => {
            "App icon missing; showing a text menu bar item instead.".into()
        }
        (Text::LauncherExited, Lang::En) => "claude-acc exited with status".into(),
        (Text::LauncherPressReturn, Lang::En) => "Press Return to close.".into(),
    }
}
