mod commands;
mod config;
mod desktop;
mod desktop_auth;
// APFS clonefile and /bin/cp -c; the whole mechanism is macOS's.
#[cfg(target_os = "macos")]
mod desktop_runtime;
mod environment;
mod i18n;
mod ide;
mod identity;
mod resolve;
mod seed;
mod sessions;

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
    /// List Claude Code sessions across accounts
    ///
    /// Shows the current directory's sessions by default. A session lives in
    /// the account it was created under, so the same id can exist as separate
    /// copies in several accounts — the newest copy of each is flagged.
    Sessions {
        /// List sessions from every project, not just the current directory
        #[arg(long)]
        all: bool,
    },
    /// Work with a single session transcript
    Session {
        #[command(subcommand)]
        action: SessionCommands,
    },
    /// Manage Claude Desktop profiles
    ///
    /// Each profile is a separate app data directory, so profiles are signed
    /// in to different accounts and — unlike CLI accounts — can run side by
    /// side. Nothing here touches the app's own profile.
    Desktop {
        #[command(subcommand)]
        action: DesktopCommands,
    },
    /// Show or set whether the claude wrapper checks `--resume` ids
    ///
    /// With the hook on, a plain `claude --resume <id>` for a session that
    /// belongs to another account offers to bring it over first. Off, it goes
    /// straight through. `claude-acc run --resume` checks either way.
    ResumeHook {
        /// on | off — omit to show the current state
        state: Option<String>,
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

#[derive(Subcommand)]
enum DesktopCommands {
    /// Create a profile and open Claude Desktop on it to sign in
    Add {
        name: String,
        /// Seed the new profile's MCP servers and preferences from the app's
        /// own profile
        #[arg(short, long)]
        seed: bool,
    },
    /// Copy claude_desktop_config.json (MCP servers, preferences) into a profile
    ///
    /// The source is the app's own profile unless `--from` names another one.
    /// An existing config is kept, not replaced, without `--force`.
    CloneConfig {
        name: String,
        /// Copy from this profile instead of the app's own
        #[arg(long)]
        from: Option<String>,
        /// Replace the profile's existing config
        #[arg(short, long)]
        force: bool,
    },
    /// Clone another profile's downloaded runtime into this one (macOS)
    ///
    /// The Cowork sandbox images, the embedded Claude Code and its VM come to
    /// ~10.5 GB and are identical between profiles. On APFS they are cloned
    /// copy-on-write, so the copy costs nothing until it diverges and saves
    /// the new profile re-downloading all of it. Per-VM identity and live
    /// caches are not copied — the app makes its own.
    #[cfg(target_os = "macos")]
    CloneRuntime {
        name: String,
        /// Clone from this profile instead of the app's own
        #[arg(long)]
        from: Option<String>,
        /// Replace runtime the profile already has
        #[arg(short, long)]
        force: bool,
    },
    /// List desktop profiles
    List,
    /// Show the account and rate-limit usage behind every profile (macOS)
    ///
    /// Needs each profile's token, so macOS asks for the login keychain
    /// password once — `list` never does.
    Usage,
    /// Open Claude Desktop on a profile
    Run { name: String },
    /// Delete a profile and everything in it
    Remove {
        /// Skip confirmation
        #[arg(short, long)]
        force: bool,
        name: String,
    },
}

#[derive(Subcommand)]
enum SessionCommands {
    /// Copy a session into another account so `claude --resume` can see it
    ///
    /// A session belongs to the account it was created under. Copying it
    /// makes a second, independent copy — the two then drift apart as each
    /// is used, which is why `claude-acc sessions` reports which copy was
    /// touched last.
    Copy {
        /// Session id (see `claude-acc sessions`)
        id: String,
        /// Destination account, or "default" for ~/.claude
        #[arg(long)]
        to: String,
        /// Source account; only needed when several accounts hold this id
        #[arg(long)]
        from: Option<String>,
        /// Skip the confirmation prompts
        #[arg(short, long)]
        force: bool,
    },
    /// Preflight a `--resume` for the generated claude wrapper (internal)
    #[command(hide = true)]
    Preflight {
        /// The arguments on their way to the real claude binary
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
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
            | Some(Commands::Session { .. })
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
        Some(Commands::Sessions { all }) => {
            std::process::exit(commands::sessions::run(&config, &i18n, all))
        }
        Some(Commands::Session { action }) => match action {
            SessionCommands::Copy {
                id,
                to,
                from,
                force,
            } => std::process::exit(commands::session::copy(
                &config,
                &i18n,
                &id,
                &to,
                from.as_deref(),
                force,
            )),
            SessionCommands::Preflight { args } => {
                std::process::exit(commands::session::preflight_hook(&config, &i18n, &args))
            }
        },
        Some(Commands::Desktop { action }) => std::process::exit(match action {
            DesktopCommands::Add { name, seed } => {
                commands::desktop::add(&config, &i18n, &name, seed)
            }
            DesktopCommands::CloneConfig { name, from, force } => {
                commands::desktop::clone_config(&config, &i18n, &name, from.as_deref(), force)
            }
            #[cfg(target_os = "macos")]
            DesktopCommands::CloneRuntime { name, from, force } => {
                commands::desktop::clone_runtime(&config, &i18n, &name, from.as_deref(), force)
            }
            DesktopCommands::List => commands::desktop::list(&config, &i18n),
            DesktopCommands::Usage => commands::desktop::usage(&config, &i18n),
            DesktopCommands::Run { name } => commands::desktop::run(&config, &i18n, &name),
            DesktopCommands::Remove { force, name } => {
                commands::desktop::remove(&config, &i18n, &name, force)
            }
        }),
        Some(Commands::ResumeHook { state }) => {
            std::process::exit(commands::resume_hook::run(&config, &i18n, state.as_deref()))
        }
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
