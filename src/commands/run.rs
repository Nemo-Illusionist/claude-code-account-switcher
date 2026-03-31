use std::process::Command;
use crate::config::AppConfig;
use crate::i18n::{I18n, Msg};

pub fn run(config: &AppConfig, i18n: &I18n, name: &str, args: &[String]) {
    if name == "default" {
        let status = Command::new("claude")
            .args(args)
            .status()
            .expect("Failed to run claude");
        std::process::exit(status.code().unwrap_or(1));
    }

    if !config.account_exists(name) {
        i18n.print(Msg::LoginNotFound(name.to_string()));
        std::process::exit(1);
    }

    let acc_dir = config.account_path(name);
    let status = Command::new("claude")
        .args(args)
        .env("CLAUDE_CONFIG_DIR", &acc_dir)
        .status()
        .expect("Failed to run claude");
    std::process::exit(status.code().unwrap_or(1));
}
