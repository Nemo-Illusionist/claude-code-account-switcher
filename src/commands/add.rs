use std::fs;
use std::process::Command;
use crate::config::{AppConfig, validate_name};
use crate::i18n::{I18n, Msg};
use crate::ide;

pub fn run(config: &AppConfig, i18n: &I18n, name: &str) {
    if name == "default" {
        i18n.print(Msg::ReservedName(name.to_string()));
        std::process::exit(1);
    }

    if !validate_name(name) {
        i18n.print(Msg::NameInvalid);
        std::process::exit(1);
    }

    let acc_dir = config.account_path(name);
    if acc_dir.is_dir() {
        i18n.print(Msg::AddExists(name.to_string()));
        std::process::exit(1);
    }

    fs::create_dir_all(&acc_dir).expect("Failed to create account directory");
    ide::ensure_account_symlink(&acc_dir).ok();
    i18n.print(Msg::AddCreated(name.to_string()));

    Command::new("claude")
        .args(["auth", "login"])
        .env("CLAUDE_CONFIG_DIR", &acc_dir)
        .status()
        .expect("Failed to run claude auth login");

    println!();
    i18n.print(Msg::AddDone);
    i18n.print(Msg::AddHintDefault(name.to_string()));
    i18n.print(Msg::AddHintLink(name.to_string()));
}
