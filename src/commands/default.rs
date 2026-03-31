use crate::config::AppConfig;
use crate::i18n::{I18n, Msg};

pub fn run(config: &AppConfig, i18n: &I18n, name: Option<&str>) {
    match name {
        None => {
            match config.get_default().ok().flatten() {
                Some(current) => i18n.print(Msg::DefaultCurrent(current)),
                None => i18n.print(Msg::DefaultStandard),
            }
        }
        Some("default") => {
            config.clear_default().expect("Failed to update config");
            i18n.print(Msg::ResetDone);
        }
        Some(name) => {
            if !config.account_exists(name) {
                i18n.print(Msg::DefaultNotFound(name.to_string()));
                super::list::run(config, i18n);
                std::process::exit(1);
            }
            config.set_default(name).expect("Failed to update config");
            i18n.print(Msg::DefaultSet(name.to_string()));
        }
    }
}
