use crate::config::AppConfig;
use crate::i18n::{I18n, Msg};

pub fn run(config: &AppConfig, i18n: &I18n) {
    let dir = std::env::current_dir().expect("Cannot get current directory");
    let dir_str = dir.to_str().expect("Invalid directory path");
    let dir_name = dir.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(dir_str);

    let removed = config.remove_link(dir_str).expect("Failed to update links");
    if !removed {
        i18n.print(Msg::UnlinkNone);
        std::process::exit(1);
    }

    i18n.print(Msg::UnlinkDone(dir_name.to_string()));
}
