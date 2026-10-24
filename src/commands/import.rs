use std::fs;
use std::path::{Path, PathBuf};

use crate::config::AppConfig;
use crate::i18n::{I18n, Msg};
use crate::identity::{self, AuditResult};
use crate::import::{self, Refusal};

// Adopt an existing Claude Code config dir (e.g. a hand-rolled `~/.claude-work`
// from the alias approach) into a managed account — without re-running
// `claude login`. The catch on macOS: Claude Code keys the OAuth token in the
// Keychain by the absolute config-dir path, so a plain copy/move would orphan
// the token. We re-key the Keychain entry to the new location; the plaintext
// `.credentials.json` fallback travels with the dir on its own.

pub fn run(config: &AppConfig, i18n: &I18n, name: &str, source: &str, move_into: bool) -> i32 {
    let src = to_absolute(source);
    if let Some(refusal) = import::refusal(config, name, &src) {
        i18n.print(match refusal {
            Refusal::Reserved => Msg::ReservedName(name.to_string()),
            Refusal::InvalidName => Msg::NameInvalid,
            Refusal::SourceNotDir => Msg::ImportSourceNotDir(source.to_string()),
            Refusal::Exists => Msg::AddExists(name.to_string()),
            Refusal::SourceManaged => Msg::ImportSourceManaged,
        });
        return 1;
    }
    let target = config.account_path(name);

    if let Some(parent) = target.parent() {
        let _ = fs::create_dir_all(parent);
    }

    if move_into {
        if let Err(e) = fs::rename(&src, &target) {
            i18n.print(Msg::ImportFailed(e.to_string()));
            return 1;
        }
    } else if let Err(e) = import::copy_tree(&src, &target) {
        let _ = fs::remove_dir_all(&target);
        i18n.print(Msg::ImportFailed(e.to_string()));
        return 1;
    }

    // Best-effort: move the Keychain token to the new path. Failure here isn't
    // fatal — the audit below tells the user whether auth actually resolved.
    let rekeyed = identity::copy_keychain_entry(&src, &target).unwrap_or(false);

    i18n.print(Msg::ImportDone(
        name.to_string(),
        target.display().to_string(),
    ));
    if rekeyed {
        i18n.print(Msg::ImportRekeyed);
    }

    // Audit confirms the imported dir resolves to a real identity (and seeds the
    // doctor cache so list/usage/status show it immediately).
    match identity::audit_account(&target) {
        AuditResult::Ok(p) => {
            let email = p.email.as_deref().unwrap_or("<unknown>");
            i18n.print(Msg::ImportVerified(email.to_string()));
            0
        }
        AuditResult::NoToken => {
            i18n.print(Msg::DoctorNoToken(name.to_string()));
            0
        }
        AuditResult::Offline => {
            i18n.print(Msg::DoctorOffline);
            0
        }
    }
}

/// Absolute form of a user-supplied path, expanding a leading `~/`. Symlinks
/// are not resolved — Claude Code hashed the path as given, so we match that.
fn to_absolute(p: &str) -> PathBuf {
    if let Some(rest) = p.strip_prefix("~/")
        && let Some(home) = dirs::home_dir()
    {
        return home.join(rest);
    }
    let path = Path::new(p);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|c| c.join(path))
            .unwrap_or_else(|_| path.to_path_buf())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_absolute_keeps_absolute() {
        // What counts as absolute differs by platform (POSIX "/..." vs Windows "C:\...").
        #[cfg(unix)]
        let p = "/tmp/foo";
        #[cfg(windows)]
        let p = r"C:\tmp\foo";
        assert_eq!(to_absolute(p), PathBuf::from(p));
    }

    #[test]
    fn to_absolute_expands_tilde() {
        if let Some(home) = dirs::home_dir() {
            assert_eq!(to_absolute("~/x"), home.join("x"));
        }
    }
}
