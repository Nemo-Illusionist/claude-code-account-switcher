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

    let standard = identity::standard_token_dir();
    match contains(&src, &accounts_dir, standard.as_deref(), move_into) {
        Some(Contains::Accounts) => {
            i18n.print(Msg::ImportSourceHoldsAccounts);
            return 1;
        }
        Some(Contains::Standard) => {
            i18n.print(Msg::ImportMoveStandard);
            return 1;
        }
        None => {}
    }

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
/// What an import source must not hold.
#[derive(Debug, PartialEq)]
enum Contains {
    /// Copying it would copy the new account into itself; moving it would
    /// move every managed account away.
    Accounts,
    /// `~/.claude` belongs to Claude Code. Copying it is a normal import;
    /// moving it takes the standard account away from under Claude Code.
    Standard,
}

/// Whether `src` holds the accounts directory, or (for a move) the standard
/// account's. Compared as physical paths, so a second spelling of the same
/// directory (a symlink, `/tmp` against `/private/tmp`) cannot slip past.
fn contains(
    src: &Path,
    accounts: &Path,
    standard: Option<&Path>,
    move_into: bool,
) -> Option<Contains> {
    let real = |p: &Path| p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
    let src = real(src);
    if real(accounts).starts_with(&src) {
        return Some(Contains::Accounts);
    }
    match standard {
        Some(standard) if move_into && real(standard).starts_with(&src) => Some(Contains::Standard),
        _ => None,
    }
}

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
    fn a_source_holding_the_accounts_or_the_standard_dir_is_refused() {
        // Regression: `import x ~` copied the home directory, accounts and
        // all, into `~/.claude-switch/accounts/x`, and `import x ~ --move`
        // could move `~/.claude` away from Claude Code.
        let base = std::env::temp_dir().join(format!("cc-import-contains-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        let home = base.join("home");
        let accounts = home.join(".claude-switch/accounts");
        let standard = home.join(".claude");
        let other = home.join(".claude-work");
        for d in [&accounts, &standard, &other] {
            fs::create_dir_all(d).unwrap();
        }
        let check = |src: &Path, move_into| contains(src, &accounts, Some(&standard), move_into);

        assert_eq!(check(&home, false), Some(Contains::Accounts));
        assert_eq!(check(&home, true), Some(Contains::Accounts));
        assert_eq!(
            check(&home.join(".claude-switch"), false),
            Some(Contains::Accounts)
        );
        assert_eq!(check(&standard, true), Some(Contains::Standard));
        // Copying the standard account is an ordinary import.
        assert_eq!(check(&standard, false), None);
        assert_eq!(check(&other, false), None);
        assert_eq!(check(&other, true), None);
        // Another spelling of the same directory.
        assert_eq!(check(&other.join(".."), false), Some(Contains::Accounts));
        fs::remove_dir_all(base).unwrap();
    }

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
