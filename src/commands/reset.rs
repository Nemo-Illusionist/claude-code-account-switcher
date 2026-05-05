use crate::config::AppConfig;
use crate::i18n::{I18n, Msg};

pub fn run(config: &AppConfig, i18n: &I18n) {
    config.clear_default().expect("Failed to update config");
    i18n.print(Msg::ResetDone);
}
