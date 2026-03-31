use std::process::Command;
use crate::config::AppConfig;
use crate::i18n::{I18n, Msg};

pub fn run(config: &AppConfig, i18n: &I18n, name: &str) {
    let acc_dir = config.account_path(name);
    if !acc_dir.is_dir() {
        i18n.print(Msg::LoginNotFound(name.to_string()));
        std::process::exit(1);
    }

    i18n.print(Msg::LoginStart(name.to_string()));
    Command::new("claude")
        .arg("login")
        .env("CLAUDE_CONFIG_DIR", &acc_dir)
        .status()
        .expect("Failed to run claude login");

    i18n.print(Msg::LoginDone);
}
