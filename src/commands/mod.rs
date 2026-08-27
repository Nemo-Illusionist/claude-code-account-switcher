pub mod activate;
pub mod add;
pub mod clone_settings;
pub mod completions;
pub mod default;
pub mod desktop;
pub mod doctor;
pub mod import;
pub mod init;
pub mod install;
pub mod link;
pub mod links;
pub mod list;
pub mod login;
pub mod remove;
pub mod reset;
pub mod resume_hook;
pub mod run;
pub mod session;
pub mod sessions;
pub mod status;
pub mod statusline;
pub mod unlink;
pub mod update;
pub mod usage;
pub mod whoami;

use crate::config::AppConfig;
use crate::i18n::{I18n, Msg};
use crate::identity;
use std::path::PathBuf;
use std::process::Command;

/// Run a prepared `claude` invocation and return its exit code.
///
/// Neither failure here is a bug in this program: `claude` may simply not be
/// installed, and on Windows an argument may contain something no `cmd.exe`
/// command line can carry. Both used to surface as a Rust panic — including
/// the `program not found` a `.cmd` shim produces, which said nothing about
/// the argument that actually caused it.
fn spawn_claude(built: Result<Command, String>, i18n: &I18n) -> i32 {
    let mut cmd = match built {
        Ok(cmd) => cmd,
        Err(reason) => {
            i18n.print(Msg::ClaudeArgUnsupported(reason));
            return 1;
        }
    };
    match cmd.status() {
        Ok(status) => status.code().unwrap_or(1),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            i18n.print(Msg::ClaudeNotFound);
            1
        }
        Err(e) => {
            i18n.print(Msg::ClaudeLaunchFailed(e.to_string()));
            1
        }
    }
}

/// `(label, cache_path)` pairs for every already-known account except the
/// one at `exclude_label` — every managed account plus the standard
/// `~/.claude` account (labeled `"~/.claude/"`). Feeds
/// `identity::find_duplicate_account`'s `known` argument, used by `add` and
/// `login` to warn when a freshly-authenticated account turns out to share
/// an identity with one that already exists.
fn known_account_cache_paths(config: &AppConfig, exclude_label: &str) -> Vec<(String, PathBuf)> {
    let mut known: Vec<(String, PathBuf)> = config
        .list_accounts()
        .unwrap_or_default()
        .into_iter()
        .filter(|acc| acc != exclude_label)
        .map(|acc| {
            let cache_path = config.account_path(&acc).join(".account-info.json");
            (acc, cache_path)
        })
        .collect();
    if exclude_label != "~/.claude/" {
        known.push((
            "~/.claude/".to_string(),
            identity::default_cache_path(&config.base_dir),
        ));
    }
    known
}

#[cfg(test)]
mod tests {
    use super::*;

    fn i18n() -> I18n {
        I18n {
            lang: crate::i18n::Lang::En,
        }
    }

    #[test]
    fn an_unrepresentable_argument_is_reported_not_panicked_on() {
        // The Windows fallback used to be `Command::new("claude")`, which on
        // a .cmd shim died with `program not found` — saying nothing about
        // the argument that was actually the problem.
        assert_eq!(
            spawn_claude(Err("argument contains a quote".to_string()), &i18n()),
            1
        );
    }

    #[test]
    fn a_missing_claude_is_reported_not_panicked_on() {
        let cmd = Command::new("no-such-binary-cc-test");
        assert_eq!(spawn_claude(Ok(cmd), &i18n()), 1);
    }

    #[test]
    fn the_exit_code_of_claude_is_passed_through() {
        let cmd = if cfg!(windows) {
            let mut c = Command::new("cmd");
            c.args(["/c", "exit 3"]);
            c
        } else {
            let mut c = Command::new("sh");
            c.args(["-c", "exit 3"]);
            c
        };
        assert_eq!(spawn_claude(Ok(cmd), &i18n()), 3);
    }
}
