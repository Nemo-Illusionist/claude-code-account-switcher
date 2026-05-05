use crate::config::{AppConfig, validate_name};
use crate::i18n::{I18n, Msg};
use std::process::Command;

pub fn run(config: &AppConfig, i18n: &I18n, name: &str) {
    if !validate_name(name) {
        i18n.print(Msg::NameInvalid);
        std::process::exit(1);
    }

    let acc_dir = config.account_path(name);
    if !acc_dir.is_dir() {
        i18n.print(Msg::LoginNotFound(name.to_string()));
        std::process::exit(1);
    }

    i18n.print(Msg::LoginStart(name.to_string()));
    Command::new("claude")
        .args(["auth", "login"])
        .env("CLAUDE_CONFIG_DIR", &acc_dir)
        .status()
        .expect("Failed to run claude auth login");

    i18n.print(Msg::LoginDone);
}
