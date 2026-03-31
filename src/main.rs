mod commands;
mod config;
mod i18n;
mod resolve;

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
    Add { name: String },
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
    /// Output shell activation code (used by shell hook)
    #[command(hide = true)]
    Activate {
        #[arg(long, default_value = "posix")]
        shell: String,
    },
    /// Output shell integration code
    Init { shell: String },
    /// Output completion data (used by shell completions)
    #[command(hide = true)]
    Completions { what: String },
}

fn main() {
    let cli = Cli::parse();
    let config = AppConfig::new();
    config.init().expect("Failed to initialize config directory");
    let i18n = I18n::new();

    match cli.command {
        None => {
            commands::list::run(&config, &i18n);
        }
        Some(Commands::List) => commands::list::run(&config, &i18n),
        Some(Commands::Add { name }) => commands::add::run(&config, &i18n, &name),
        Some(Commands::Login { name }) => commands::login::run(&config, &i18n, &name),
        Some(Commands::Remove { force, name }) => commands::remove::run(&config, &i18n, &name, force),
        Some(Commands::Default { name }) => commands::default::run(&config, &i18n, name.as_deref()),
        Some(Commands::Reset) => commands::reset::run(&config, &i18n),
        Some(Commands::Link { name }) => commands::link::run(&config, &i18n, &name),
        Some(Commands::Unlink) => commands::unlink::run(&config, &i18n),
        Some(Commands::Links) => commands::links::run(&config, &i18n),
        Some(Commands::Status) => commands::status::run(&config, &i18n),
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
