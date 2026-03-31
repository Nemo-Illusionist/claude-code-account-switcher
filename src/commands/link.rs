use crate::config::AppConfig;
use crate::i18n::{I18n, Msg};

pub fn run(config: &AppConfig, i18n: &I18n, name: &str) {
    if name != "default" && !config.account_exists(name) {
        i18n.print(Msg::LinkNotFound(name.to_string()));
        super::list::run(config, i18n);
        std::process::exit(1);
    }

    let dir = std::env::current_dir().expect("Cannot get current directory");
    let dir_str = dir.to_str().expect("Invalid directory path");
    let dir_name = dir.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(dir_str);

    config.set_link(dir_str, name).expect("Failed to update links");

    if name == "default" {
        i18n.print(Msg::LinkDoneDefault(dir_name.to_string()));
    } else {
        i18n.print(Msg::LinkDone(dir_name.to_string(), name.to_string()));
    }
}
