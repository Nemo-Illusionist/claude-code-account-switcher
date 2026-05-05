use crate::config::{AppConfig, validate_name};
use crate::i18n::{I18n, Msg};
use crate::ide;
use crate::seed;
use std::fs;
use std::process::Command;

pub fn run(config: &AppConfig, i18n: &I18n, name: &str, seed_from_default: bool) {
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

    // Seed before printing AddCreated, so the copy report is logically
    // attached to "what the new account got". Errors here are non-fatal —
    // an empty account dir is still usable.
    if seed_from_default {
        match seed::copy_user_config(&acc_dir) {
            Ok(report) if report.is_empty() => {
                i18n.print(Msg::SeedNothingToCopy);
            }
            Ok(report) => {
                for entry in &report.copied {
                    i18n.print(Msg::SeedCopied(entry.clone()));
                }
            }
            Err(e) => eprintln!("seed: {}", e),
        }
    }

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
