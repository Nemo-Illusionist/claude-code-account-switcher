use std::fs;
use std::path::{Path, PathBuf};

use crate::config::{AppConfig, validate_name};
use crate::i18n::{I18n, Msg};
use crate::identity::{self, AuditResult};

// Adopt an existing Claude Code config dir (e.g. a hand-rolled `~/.claude-work`
// from the alias approach) into a managed account — without re-running
// `claude login`. The catch on macOS: Claude Code keys the OAuth token in the
// Keychain by the absolute config-dir path, so a plain copy/move would orphan
// the token. We re-key the Keychain entry to the new location; the plaintext
// `.credentials.json` fallback travels with the dir on its own.

pub fn run(config: &AppConfig, i18n: &I18n, name: &str, source: &str, move_into: bool) -> i32 {
    if name == "default" {
        i18n.print(Msg::ReservedName(name.to_string()));
        return 1;
    }
    if !validate_name(name) {
        i18n.print(Msg::NameInvalid);
        return 1;
    }

    let src = to_absolute(source);
    if !src.is_dir() {
        i18n.print(Msg::ImportSourceNotDir(source.to_string()));
        return 1;
    }

    let target = config.account_path(name);
    if target.exists() {
        i18n.print(Msg::AddExists(name.to_string()));
        return 1;
    }
    // Refuse to import a dir that's already under our accounts/ tree — that's
    // not an import, and the move variant would be destructive.
    let accounts_dir = config.base_dir.join("accounts");
    if src.starts_with(&accounts_dir) {
        i18n.print(Msg::ImportSourceManaged);
        return 1;
    }

    if let Some(parent) = target.parent() {
        let _ = fs::create_dir_all(parent);
    }

    if move_into {
        if let Err(e) = fs::rename(&src, &target) {
            i18n.print(Msg::ImportFailed(e.to_string()));
            return 1;
        }
    } else if let Err(e) = copy_tree(&src, &target) {
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

/// Recursively copy `src` to `dst`, preserving symlinks (the per-account
/// `ide/` link in particular shouldn't be dereferenced into a fat copy).
fn copy_tree(src: &Path, dst: &Path) -> std::io::Result<()> {
    let meta = fs::symlink_metadata(src)?;
    let ft = meta.file_type();
    if ft.is_symlink() {
        copy_symlink(src, dst)
    } else if ft.is_dir() {
        fs::create_dir_all(dst)?;
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            copy_tree(&entry.path(), &dst.join(entry.file_name()))?;
        }
        Ok(())
    } else {
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(src, dst).map(|_| ())
    }
}

#[cfg(unix)]
fn copy_symlink(src: &Path, dst: &Path) -> std::io::Result<()> {
    let target = fs::read_link(src)?;
    std::os::unix::fs::symlink(target, dst)
}

#[cfg(not(unix))]
fn copy_symlink(src: &Path, dst: &Path) -> std::io::Result<()> {
    // Windows config dirs don't carry our symlinks; fall back to a file copy.
    fs::copy(src, dst).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_absolute_keeps_absolute() {
        assert_eq!(to_absolute("/tmp/foo"), PathBuf::from("/tmp/foo"));
    }

    #[test]
    fn to_absolute_expands_tilde() {
        if let Some(home) = dirs::home_dir() {
            assert_eq!(to_absolute("~/x"), home.join("x"));
        }
    }

    #[test]
    fn copy_tree_copies_files_and_symlinks() {
        let base = std::env::temp_dir().join(format!("cc-import-test-{}", std::process::id()));
        let src = base.join("src");
        let dst = base.join("dst");
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(src.join("sub")).unwrap();
        fs::write(src.join("a.txt"), b"hello").unwrap();
        fs::write(src.join("sub/b.txt"), b"world").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink("a.txt", src.join("link")).unwrap();

        copy_tree(&src, &dst).unwrap();

        assert_eq!(fs::read(dst.join("a.txt")).unwrap(), b"hello");
        assert_eq!(fs::read(dst.join("sub/b.txt")).unwrap(), b"world");
        #[cfg(unix)]
        assert!(
            fs::symlink_metadata(dst.join("link"))
                .unwrap()
                .file_type()
                .is_symlink()
        );

        let _ = fs::remove_dir_all(&base);
    }
}
