use crate::config::{AppConfig, validate_name};
use crate::i18n::{I18n, Msg};
use crate::seed;

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

    match seed::copy_user_config(&acc_dir) {
        Ok(report) if report.is_empty() => {
            i18n.print(Msg::SeedNothingToCopy);
        }
        Ok(report) => {
            for entry in &report.copied {
                i18n.print(Msg::SeedCopied(entry.clone()));
            }
        }
        Err(e) => {
            eprintln!("clone-settings: {}", e);
            std::process::exit(1);
        }
    }
}
