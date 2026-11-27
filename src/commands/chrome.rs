use std::path::PathBuf;

use crate::chrome::{self, SetError};
use crate::config::{AppConfig, is_reserved_name, validate_name};
use crate::i18n::{I18n, Msg};
use crate::identity;

/// `claude-acc chrome [<account>] [--on|--off]`.
///
/// Claude Code keeps the Claude in Chrome switch per config dir, so a new
/// account has the browser tools missing with nothing on screen to say why —
/// which `doctor` reports and, until now, could only tell you to fix by
/// hand, by starting a session in that account and running `/chrome`. That
/// is the only thing in this tool's reach that needed an interactive
/// session to set.
///
/// With no flag it reports, which is the same reader `doctor` uses.
pub fn run(config: &AppConfig, i18n: &I18n, name: Option<&str>, on: Option<bool>) -> i32 {
    let targets = match name {
        Some(name) => match one(config, name) {
            Some(target) => vec![target],
            None => {
                if is_reserved_name(name) {
                    // `default` is a real account here, just not a managed
                    // directory — `standard_token_dir` is what knows where
                    // it lives.
                    i18n.print(Msg::ChromeNoStandardAccount);
                } else if !validate_name(name) {
                    i18n.print(Msg::NameInvalid);
                } else {
                    i18n.print(Msg::LoginNotFound(name.to_string()));
                }
                return 1;
            }
        },
        None => all(config),
    };
    if targets.is_empty() {
        i18n.print(Msg::ListEmpty);
        return 0;
    }

    match on {
        None => {
            for (label, dir) in &targets {
                i18n.print(Msg::ChromeState(
                    label.clone(),
                    state_word(&chrome::enabled(dir)),
                ));
            }
            0
        }
        Some(on) => {
            let mut failed = false;
            for (label, dir) in &targets {
                match chrome::set_enabled(dir, on) {
                    Ok(()) => i18n.print(Msg::ChromeSet(label.clone(), on)),
                    Err(SetError::Refused(said)) => {
                        i18n.print(Msg::ChromeRefused(label.clone(), said));
                        failed = true;
                    }
                    Err(SetError::NotStarted(why)) => {
                        i18n.print(Msg::ChromeClaudeMissing(why));
                        return 1;
                    }
                }
            }
            if failed { 1 } else { 0 }
        }
    }
}

/// `on` / `off` / `unknown`, for a message. `None` means no `.claude.json`
/// yet, or one that does not parse — a brand-new account, not an account
/// that said no.
fn state_word(enabled: &Option<bool>) -> String {
    match enabled {
        Some(true) => "on".to_string(),
        Some(false) => "off".to_string(),
        None => "unknown".to_string(),
    }
}

/// The one account a name designates, with the label to print for it.
fn one(config: &AppConfig, name: &str) -> Option<(String, PathBuf)> {
    if is_reserved_name(name) {
        return identity::standard_token_dir().map(|dir| (STANDARD.to_string(), dir));
    }
    if !validate_name(name) {
        return None;
    }
    let dir = config.account_path(name);
    dir.is_dir().then(|| (name.to_string(), dir))
}

/// Every account, managed ones first, then the standard one when it exists.
fn all(config: &AppConfig) -> Vec<(String, PathBuf)> {
    let mut out: Vec<(String, PathBuf)> = config
        .list_accounts()
        .unwrap_or_default()
        .into_iter()
        .map(|name| {
            let dir = config.account_path(&name);
            (name, dir)
        })
        .collect();
    if let Some(dir) = identity::standard_token_dir() {
        out.push((STANDARD.to_string(), dir));
    }
    out
}

const STANDARD: &str = "~/.claude/";

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_config(tag: &str) -> AppConfig {
        let base = std::env::temp_dir().join(format!("cc-chrome-{}-{}", tag, std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let config = AppConfig { base_dir: base };
        config.init().unwrap();
        config
    }

    #[test]
    fn a_name_designates_one_account_or_nothing() {
        let c = temp_config("one");
        let work = c.accounts_dir().join("work");
        std::fs::create_dir_all(&work).unwrap();

        assert_eq!(one(&c, "work"), Some(("work".to_string(), work)));
        // Not an account here, a name no command takes, and — whatever the
        // machine's standard dir turns out to be — never `accounts/default`.
        assert_eq!(one(&c, "nope"), None);
        assert_eq!(one(&c, "work.lock"), None);
        assert!(one(&c, "default").is_none_or(|(label, dir)| {
            label == "~/.claude/" && dir != c.accounts_dir().join("default")
        }));
        let _ = std::fs::remove_dir_all(&c.base_dir);
    }

    #[test]
    fn the_state_word_separates_off_from_unknown() {
        // A config dir Claude Code has never written is a new account, not
        // an account that said no — `doctor` makes the same distinction, and
        // reporting a fresh account as "off" would be a finding about
        // nothing.
        assert_eq!(state_word(&Some(true)), "on");
        assert_eq!(state_word(&Some(false)), "off");
        assert_eq!(state_word(&None), "unknown");
    }

    #[test]
    fn every_account_is_listed_with_the_standard_one_last() {
        let c = temp_config("all");
        for n in ["work", "work-2"] {
            std::fs::create_dir_all(c.accounts_dir().join(n)).unwrap();
        }
        let listed = all(&c);
        let names: Vec<&str> = listed.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(&names[..2], &["work", "work-2"]);
        // The standard account is appended when this machine has one; its
        // presence depends on the environment, so only the order is pinned.
        assert!(names.len() == 2 || (names.len() == 3 && names[2] == "~/.claude/"));
        let _ = std::fs::remove_dir_all(&c.base_dir);
    }
}
