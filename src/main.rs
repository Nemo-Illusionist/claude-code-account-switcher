mod commands;
mod config;
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

fn main() {
    let cli = Cli::parse();
    let config = AppConfig::new();
    config
        .init()
        .expect("Failed to initialize config directory");
    let i18n = I18n::new();

    match cli.command {
        None => {
            commands::list::run(&config, &i18n);
        }
        Some(Commands::List) => commands::list::run(&config, &i18n),
        Some(Commands::Add { name, seed }) => commands::add::run(&config, &i18n, &name, seed),
        Some(Commands::CloneSettings { name }) => {
            commands::clone_settings::run(&config, &i18n, &name)
        }
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
        Some(Commands::Run { name, args }) => commands::run::run(&config, &i18n, &name, &args),
        Some(Commands::Doctor { json }) => {
            std::process::exit(commands::doctor::run(&config, &i18n, json))
        }
        Some(Commands::Whoami) => commands::whoami::run(&config),
        Some(Commands::Install) => commands::install::run(&config, &i18n),
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
}
