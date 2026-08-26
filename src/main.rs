mod commands;
mod config;
mod environment;
mod i18n;
mod ide;
mod identity;
mod resolve;
mod seed;

use clap::{Parser, Subcommand};
use commands::activate::ShellSyntax;
use config::AppConfig;
use i18n::I18n;

#[derive(Parser)]
#[command(name = "claude-acc", version, about = "Claude Code Account Switcher")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// List all accounts
    List,
    /// Add account (runs claude login)
    Add {
        name: String,
        /// Seed the new account dir with settings.json / CLAUDE.md / agents
        /// / commands / output-styles / skills from your standard ~/.claude/
        #[arg(short, long)]
        seed: bool,
    },
    /// Seed an existing account dir from ~/.claude/
    ///
    /// Copies settings.json, CLAUDE.md, agents/, commands/, output-styles/,
    /// and skills/. Skips files that already exist; never overwrites.
    CloneSettings { name: String },
    /// Import an existing Claude config dir as an account (no re-login)
    ///
    /// Copies (or moves, with --move) the directory into the managed
    /// location and, on macOS, re-keys its Keychain token so auth is kept.
    Import {
        name: String,
        /// Path to the existing config dir (e.g. ~/.claude-work)
        source: String,
        /// Move the directory instead of copying it
        #[arg(long = "move")]
        move_into: bool,
    },
    /// Re-login to an account
    Login { name: String },
    /// Remove account
    Remove {
        /// Skip confirmation
        #[arg(short, long)]
        force: bool,
        name: String,
    },
    /// Show/set default account
    Default { name: Option<String> },
    /// Reset default to ~/.claude/
    Reset,
    /// Link account to current directory
    Link { name: String },
    /// Unlink current directory
    Unlink,
    /// Show all directory links
    Links,
    /// Show active account
    Status,
    /// Show usage (5h / 7d rate-limit windows) for every account
    Usage,
    /// Render the Claude Code status line (reads session JSON on stdin)
    ///
    /// Wire it into Claude Code's `statusLine` setting. Run with `--install`
    /// to write that config into the active account's settings.json for you.
    Statusline {
        /// Install the statusLine config into the active account's settings.json
        #[arg(long)]
        install: bool,
    },
    /// Run claude under a specific account
    Run {
        name: String,
        /// Extra arguments passed to claude
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Audit each account's actual OAuth identity (email, UUID)
    Doctor {
        /// Output as JSON (suitable for scripting)
        #[arg(long)]
        json: bool,
    },
    /// Print the email or account name of the active account
    Whoami,
    /// Install binary and shell integration
    Install,
    /// Update the installed binary to the latest GitHub release
    Update {
        /// Only check whether an update is available; don't download
        #[arg(long)]
        check: bool,
        /// Install a specific version instead of the latest (e.g. 0.10.5) —
        /// for rolling back after a bad release
        #[arg(long)]
        version: Option<String>,
    },
    /// Output shell activation code (used by shell hook)
    #[command(hide = true)]
    Activate {
        #[arg(long, default_value = "posix")]
        shell: String,
    },
    /// Output shell integration code (used internally by eval)
    #[command(hide = true)]
    Init { shell: String },
    /// Output completion data (used by shell completions)
    #[command(hide = true)]
    Completions { what: String },
}

/// Whether `command` is safe to follow with the passive "update available"
/// hint. Excluded: `Activate`/`Init`/`Completions` (their stdout is `eval`'d
/// or otherwise machine-parsed by the shell integration — extra text would
/// break it, not just look untidy), `Statusline` (rendered inside Claude
/// Code's own UI), `Doctor`/`Update` (may run with `--json`, or is already
/// about updating), and `Run` (hands the terminal to an interactive `claude`
/// session that can run for hours — a hint printed before it starts would be
/// stale by the time anyone sees it).
fn should_show_update_hint(command: &Option<Commands>) -> bool {
    !matches!(
        command,
        Some(Commands::Activate { .. })
            | Some(Commands::Init { .. })
            | Some(Commands::Completions { .. })
            | Some(Commands::Statusline { .. })
            | Some(Commands::Doctor { .. })
            | Some(Commands::Update { .. })
            | Some(Commands::Run { .. })
    )
}

fn main() {
    let cli = Cli::parse();
    let config = AppConfig::new();
    config
        .init()
        .expect("Failed to initialize config directory");
    let i18n = I18n::new();
    let show_hint = should_show_update_hint(&cli.command);

    match cli.command {
        None => {
            commands::list::run(&config, &i18n);
        }
        Some(Commands::List) => commands::list::run(&config, &i18n),
        Some(Commands::Add { name, seed }) => commands::add::run(&config, &i18n, &name, seed),
        Some(Commands::CloneSettings { name }) => {
            commands::clone_settings::run(&config, &i18n, &name)
        }
        Some(Commands::Import {
            name,
            source,
            move_into,
        }) => std::process::exit(commands::import::run(
            &config, &i18n, &name, &source, move_into,
        )),
        Some(Commands::Login { name }) => commands::login::run(&config, &i18n, &name),
        Some(Commands::Remove { force, name }) => {
            commands::remove::run(&config, &i18n, &name, force)
        }
        Some(Commands::Default { name }) => commands::default::run(&config, &i18n, name.as_deref()),
        Some(Commands::Reset) => commands::reset::run(&config, &i18n),
        Some(Commands::Link { name }) => commands::link::run(&config, &i18n, &name),
        Some(Commands::Unlink) => commands::unlink::run(&config, &i18n),
        Some(Commands::Links) => commands::links::run(&config, &i18n),
        Some(Commands::Status) => commands::status::run(&config, &i18n),
        Some(Commands::Usage) => commands::usage::run(&config, &i18n),
        Some(Commands::Statusline { install }) => {
            std::process::exit(commands::statusline::run(&config, &i18n, install))
        }
        Some(Commands::Run { name, args }) => commands::run::run(&config, &i18n, &name, &args),
        Some(Commands::Doctor { json }) => {
            std::process::exit(commands::doctor::run(&config, &i18n, json))
        }
        Some(Commands::Whoami) => commands::whoami::run(&config),
        Some(Commands::Install) => commands::install::run(&config, &i18n),
        Some(Commands::Update { check, version }) => std::process::exit(commands::update::run(
            &config,
            &i18n,
            check,
            version.as_deref(),
        )),
        Some(Commands::Activate { shell }) => {
            let syntax = match shell.as_str() {
                "powershell" | "pwsh" => ShellSyntax::PowerShell,
                _ => ShellSyntax::Posix,
            };
            commands::activate::run(&config, syntax);
        }
        Some(Commands::Init { shell }) => commands::init::run(&shell),
        Some(Commands::Completions { what }) => commands::completions::run(&config, &what),
    }

    // Only reached by commands that don't std::process::exit internally —
    // which happens to already exclude Run/Import/error paths too, on top of
    // the explicit exclusions in should_show_update_hint.
    if show_hint {
        commands::update::maybe_print_hint(&config, &i18n);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_hint_excludes_eval_consumed_commands() {
        assert!(!should_show_update_hint(&Some(Commands::Activate {
            shell: "posix".to_string()
        })));
        assert!(!should_show_update_hint(&Some(Commands::Init {
            shell: "zsh".to_string()
        })));
        assert!(!should_show_update_hint(&Some(Commands::Completions {
            what: "accounts".to_string()
        })));
    }

    #[test]
    fn update_hint_excludes_statusline_doctor_update_run() {
        assert!(!should_show_update_hint(&Some(Commands::Statusline {
            install: false
        })));
        assert!(!should_show_update_hint(&Some(Commands::Doctor {
            json: false
        })));
        assert!(!should_show_update_hint(&Some(Commands::Update {
            check: false,
            version: None
        })));
        assert!(!should_show_update_hint(&Some(Commands::Run {
            name: "work".to_string(),
            args: vec![]
        })));
    }

    #[test]
    fn update_hint_shown_for_ordinary_commands() {
        assert!(should_show_update_hint(&None));
        assert!(should_show_update_hint(&Some(Commands::List)));
        assert!(should_show_update_hint(&Some(Commands::Whoami)));
        assert!(should_show_update_hint(&Some(Commands::Status)));
    }
}
