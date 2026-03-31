use std::fs;
use std::process::Command;
use crate::config::AppConfig;
use crate::i18n::{I18n, Msg};

pub fn run(config: &AppConfig, i18n: &I18n, name: &str) {
    if name == "default" {
        i18n.print(Msg::ReservedName(name.to_string()));
        std::process::exit(1);
    }

    let acc_dir = config.account_path(name);
    if acc_dir.is_dir() {
        i18n.print(Msg::AddExists(name.to_string()));
        std::process::exit(1);
    }

    fs::create_dir_all(&acc_dir).expect("Failed to create account directory");
    i18n.print(Msg::AddCreated(name.to_string()));

    Command::new("claude")
        .arg("login")
        .env("CLAUDE_CONFIG_DIR", &acc_dir)
        .status()
        .expect("Failed to run claude login");

    println!();
    i18n.print(Msg::AddDone);
    i18n.print(Msg::AddHintDefault(name.to_string()));
    i18n.print(Msg::AddHintLink(name.to_string()));
}
