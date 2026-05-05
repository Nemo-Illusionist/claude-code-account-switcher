// IDE integration: wrapper script + per-account `ide/` symlinks.
//
// Problem: IDEs (PhpStorm, IntelliJ, VSCode) launch the `claude` binary
// without sourcing the user's shell config, so `CLAUDE_CONFIG_DIR` would
// not be set and the wrong account would be used. Additionally Claude
// Code writes IDE lock files to `$CLAUDE_CONFIG_DIR/ide/`, but IDE
// plugins always look in `~/.claude/ide/`.
//
// Fix:
// 1. Install a `claude` wrapper at `~/.claude-switch/bin/claude` which
//    invokes `claude-acc activate` to set CLAUDE_CONFIG_DIR for $PWD,
//    then exec's the real claude binary. The shell init prepends this
//    bin dir to PATH so terminals + IDEs both pick up the wrapper.
// 2. Symlink `~/.claude-switch/accounts/<name>/ide → ~/.claude/ide` for
//    every account so both sides agree on lock file location.

use std::fs;
use std::io;
use std::path::Path;

use crate::config::AppConfig;

#[cfg(not(windows))]
const WRAPPER_TEMPLATE: &str = include_str!("../shell/claude-wrapper.sh");
#[cfg(not(windows))]
const WRAPPER_PLACEHOLDER: &str = "__CLAUDE_ACC_BIN__";

/// `~/.claude/ide` — the canonical IDE lock-file directory.
pub fn shared_ide_dir() -> Option<std::path::PathBuf> {
    dirs::home_dir().map(|h| h.join(".claude/ide"))
}

/// Ensure `acc_dir/ide` is a symlink to `~/.claude/ide`. If it's a real
/// (possibly non-empty) directory, leave it alone — caller can decide
/// whether to migrate. Returns Ok if the link is in place after the call.
pub fn ensure_account_symlink(acc_dir: &Path) -> io::Result<()> {
    let Some(target) = shared_ide_dir() else {
        return Ok(());
    };
    fs::create_dir_all(&target)?;
    let link = acc_dir.join("ide");

    match fs::symlink_metadata(&link) {
        Ok(meta) if meta.file_type().is_symlink() => Ok(()),
        Ok(meta) if meta.is_dir() => {
            // Real directory: only auto-migrate if it's empty (just stale
            // lock files would normally be in there, but be conservative).
            let empty = fs::read_dir(&link)
                .map(|mut d| d.next().is_none())
                .unwrap_or(false);
            if empty {
                fs::remove_dir(&link)?;
                symlink(&target, &link)
            } else {
                Ok(())
            }
        }
        Ok(_) => {
            fs::remove_file(&link)?;
            symlink(&target, &link)
        }
        Err(_) => symlink(&target, &link),
    }
}

/// Refresh `ide/` symlinks for all existing accounts. Used by `install`.
pub fn refresh_all_account_symlinks(config: &AppConfig) -> io::Result<()> {
    for acc in config.list_accounts()? {
        let acc_dir = config.account_path(&acc);
        ensure_account_symlink(&acc_dir).ok();
    }
    Ok(())
}

/// Write/update `~/.claude-switch/bin/claude` wrapper. Always overwrites
/// — the cost is one fs::write per `install` call. Returns the wrapper
/// path. Skipped on Windows (IDEs there don't share this PATH model and
/// the wrapper script would not run as a `.exe`).
#[cfg(not(windows))]
pub fn install_wrapper(
    config: &AppConfig,
    claude_acc_bin: &Path,
) -> io::Result<std::path::PathBuf> {
    use std::os::unix::fs::PermissionsExt;
    let bin_dir = config.base_dir.join("bin");
    fs::create_dir_all(&bin_dir)?;
    let wrapper = bin_dir.join("claude");
    let bin_str = claude_acc_bin.to_string_lossy();
    let content = WRAPPER_TEMPLATE.replace(WRAPPER_PLACEHOLDER, &bin_str);
    fs::write(&wrapper, content)?;
    fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o755))?;
    Ok(wrapper)
}

#[cfg(windows)]
pub fn install_wrapper(
    _config: &AppConfig,
    _claude_acc_bin: &Path,
) -> io::Result<std::path::PathBuf> {
    // Windows: no shell-script wrapper. PATH-based IDE integration would
    // need a .cmd or .exe shim — out of scope for this iteration.
    Ok(std::path::PathBuf::new())
}

#[cfg(unix)]
fn symlink(target: &Path, link: &Path) -> io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}

#[cfg(windows)]
fn symlink(_target: &Path, _link: &Path) -> io::Result<()> {
    // Windows symlinks need elevated privileges by default and the IDE
    // wrapper isn't installed there anyway. Skip silently.
    Ok(())
}
