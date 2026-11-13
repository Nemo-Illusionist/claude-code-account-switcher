//! Adopting an existing Claude Code config dir as a managed account: which
//! sources and names are refused, and the copy. The CLI's `import` and the
//! manager share both.

use std::fs;
use std::path::Path;

use crate::config::{AppConfig, is_reserved_name, validate_name};

/// Why `import` will not take a source, in the order the checks run.
#[derive(Debug, PartialEq)]
pub enum Refusal {
    Reserved,
    InvalidName,
    SourceNotDir,
    Exists,
    /// Already under our `accounts/` tree. That is not an import, and the
    /// move variant would be destructive.
    SourceManaged,
}

/// Whether `src`, an absolute path, may be imported as account `name`.
pub fn refusal(config: &AppConfig, name: &str, src: &Path) -> Option<Refusal> {
    if is_reserved_name(name) {
        return Some(Refusal::Reserved);
    }
    if !validate_name(name) {
        return Some(Refusal::InvalidName);
    }
    if !src.is_dir() {
        return Some(Refusal::SourceNotDir);
    }
    if config.account_path(name).exists() {
        return Some(Refusal::Exists);
    }
    if src.starts_with(config.accounts_dir()) {
        return Some(Refusal::SourceManaged);
    }
    None
}

/// Recursively copy `src` to `dst`, preserving symlinks (the per-account
/// `ide/` link in particular shouldn't be dereferenced into a fat copy).
pub fn copy_tree(src: &Path, dst: &Path) -> std::io::Result<()> {
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

    #[test]
    fn each_refusal_is_reported_in_the_order_the_cli_checks() {
        let base = std::env::temp_dir().join(format!("cc-import-refusal-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        let config = AppConfig {
            base_dir: base.join("switch"),
        };
        config.init().unwrap();
        fs::create_dir_all(config.account_path("work")).unwrap();
        let outside = base.join("claude-work");
        fs::create_dir_all(&outside).unwrap();
        let missing = base.join("missing");

        assert_eq!(refusal(&config, "fresh", &outside), None);
        assert_eq!(
            refusal(&config, "default", &outside),
            Some(Refusal::Reserved)
        );
        assert_eq!(
            refusal(&config, "a b", &missing),
            Some(Refusal::InvalidName)
        );
        assert_eq!(
            refusal(&config, "work", &missing),
            Some(Refusal::SourceNotDir)
        );
        assert_eq!(refusal(&config, "work", &outside), Some(Refusal::Exists));
        assert_eq!(
            refusal(&config, "fresh", &config.account_path("work")),
            Some(Refusal::SourceManaged)
        );
        let _ = fs::remove_dir_all(&base);
    }
}
