use crate::config::AppConfig;
use crate::resolve;

#[derive(Clone, Copy)]
pub enum ShellSyntax {
    Posix,
    PowerShell,
}

pub fn run(config: &AppConfig, shell: ShellSyntax) {
    let cwd = std::env::current_dir().expect("Cannot get current directory");
    let account = resolve::resolve_account(config, &cwd);

    match account.as_deref() {
        Some("default") | None => match shell {
            ShellSyntax::Posix => println!("unset CLAUDE_CONFIG_DIR"),
            ShellSyntax::PowerShell => println!("Remove-Item Env:\\CLAUDE_CONFIG_DIR -ErrorAction SilentlyContinue"),
        },
        Some(name) => {
            let path = config.account_path(name);
            if path.is_dir() {
                match shell {
                    ShellSyntax::Posix => println!("export CLAUDE_CONFIG_DIR='{}'", path.display()),
                    ShellSyntax::PowerShell => println!("$env:CLAUDE_CONFIG_DIR = '{}'", path.display()),
                }
            } else {
                match shell {
                    ShellSyntax::Posix => println!("unset CLAUDE_CONFIG_DIR"),
                    ShellSyntax::PowerShell => println!("Remove-Item Env:\\CLAUDE_CONFIG_DIR -ErrorAction SilentlyContinue"),
                }
            }
        }
    }
}
