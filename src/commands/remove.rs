use crate::config::{AppConfig, is_reserved_name, validate_name};
use crate::i18n::{I18n, Msg};
use std::fs;
use std::io::{self, Write};

pub fn run(config: &AppConfig, i18n: &I18n, name: &str, force: bool, purge: bool) {
    if is_reserved_name(name) {
        i18n.print(Msg::ReservedName(name.to_string()));
        std::process::exit(1);
    }

    if !validate_name(name) {
        i18n.print(Msg::NameInvalid);
        std::process::exit(1);
    }

    let acc_dir = config.account_path(name);
    if !acc_dir.is_dir() {
        i18n.print(Msg::RemoveNotFound(name.to_string()));
        std::process::exit(1);
    }

    if !force {
        // The question names the outcome, because the two are not equally
        // recoverable and the difference is the whole point of the flag.
        let prompt = if purge {
            Msg::RemovePurgeConfirm(name.to_string())
        } else {
            Msg::RemoveConfirm(name.to_string())
        };
        print!("{}", i18n.msg(prompt));
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
    if let Ok(Some(ref def)) = config.get_default()
        && def == name
    {
        config.clear_default().ok();
    }

    // Remove links for this account
    config.remove_links_for_account(name).ok();

    // `--purge` is an explicit request for the directory to be gone: the
    // Trash keeps holding the disk space, and an account dir is exactly the
    // kind of thing someone may want off the machine rather than in a bin.
    if purge {
        fs::remove_dir_all(&acc_dir).expect("Failed to remove account directory");
        i18n.print(Msg::RemoveDeleted(name.to_string()));
        return;
    }

    // Otherwise to the Trash rather than gone: an account dir holds
    // transcripts, settings and plugins that exist nowhere else, so getting
    // this wrong should cost a drag back out, not a restore from backup.
    //
    // Deleting outright stays the fallback — a trash on another filesystem,
    // or a platform without one — because the user asked for the account to
    // go and a half-removal is worse than either outcome. Which of the two
    // happened is always said out loud.
    match crate::trash::trash_dir(&acc_dir) {
        Ok(dest) => i18n.print(Msg::RemoveTrashed(
            name.to_string(),
            dest.display().to_string(),
        )),
        Err(_) => {
            fs::remove_dir_all(&acc_dir).expect("Failed to remove account directory");
            i18n.print(Msg::RemoveDeleted(name.to_string()));
        }
    }
}
