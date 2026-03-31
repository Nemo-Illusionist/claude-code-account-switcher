use std::fs;
use std::io::{self, Write};
use crate::config::AppConfig;
use crate::i18n::{I18n, Msg};

pub fn run(config: &AppConfig, i18n: &I18n, name: &str, force: bool) {
    if name == "default" {
        i18n.print(Msg::ReservedName(name.to_string()));
        std::process::exit(1);
    }

    let acc_dir = config.account_path(name);
    if !acc_dir.is_dir() {
        i18n.print(Msg::RemoveNotFound(name.to_string()));
        std::process::exit(1);
    }

    if !force {
        print!("{}", i18n.msg(Msg::RemoveConfirm(name.to_string())));
        io::stdout().flush().unwrap();
        let mut reply = String::new();
        io::stdin().read_line(&mut reply).unwrap();
        let reply = reply.trim().to_lowercase();
        if !reply.starts_with('y') && !reply.starts_with('д') {
            i18n.print(Msg::RemoveCancelled);
            std::process::exit(1);
        }
    }

    // Clear default if it was this account
    if let Ok(Some(ref def)) = config.get_default() {
        if def == name {
            config.clear_default().ok();
        }
    }

    // Remove links for this account
    config.remove_links_for_account(name).ok();

    fs::remove_dir_all(&acc_dir).expect("Failed to remove account directory");
    i18n.print(Msg::RemoveDeleted(name.to_string()));
}
