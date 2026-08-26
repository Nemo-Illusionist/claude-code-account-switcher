// Claude Desktop profiles.
//
// The desktop app is Electron, so it honours Chromium's `--user-data-dir`
// switch. Pointing it at a directory of our own gives a fully isolated app
// profile — the same move this tool already makes for the CLI with
// `CLAUDE_CONFIG_DIR`, and unlike the CLI accounts these can run at the same
// time: the app takes no single-instance lock, and Chromium's own lock lives
// inside each user-data-dir.
//
// Nothing is copied and nothing is killed, so none of the "swap the signed-in
// state on disk" problems apply — see issue #75 for the research.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::config::AppConfig;

/// Overrides the located `Claude.app`. For testing, and for anyone whose app
/// lives somewhere the search below doesn't reach.
pub const APP_ENV: &str = "CLAUDE_ACC_DESKTOP_APP";

/// `~/.claude-switch/desktop/` — one subdirectory per profile, alongside
/// `accounts/`.
pub fn profiles_dir(config: &AppConfig) -> PathBuf {
    config.base_dir.join("desktop")
}

pub fn profile_path(config: &AppConfig, name: &str) -> PathBuf {
    profiles_dir(config).join(name)
}

pub fn profile_exists(config: &AppConfig, name: &str) -> bool {
    profile_path(config, name).is_dir()
}

pub fn list_profiles(config: &AppConfig) -> io::Result<Vec<String>> {
    let dir = profiles_dir(config);
    if !dir.exists() {
        return Ok(vec![]);
    }
    let mut names = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir()
            && let Some(name) = entry.file_name().to_str()
        {
            names.push(name.to_string());
        }
    }
    names.sort();
    Ok(names)
}

/// The app's own, unmanaged profile — the one it uses when launched normally.
/// Shown for orientation; never written to.
pub fn standard_profile() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        dirs::home_dir().map(|h| h.join("Library/Application Support/Claude"))
    }
    #[cfg(not(target_os = "macos"))]
    {
        None
    }
}

/// Whether this platform can launch the desktop app yet. Locating the
/// executable is the whole of the per-platform work — Windows and Linux are
/// tracked separately in #75.
pub fn supported() -> bool {
    cfg!(target_os = "macos")
}

/// The installed `Claude.app`, or `None` if it isn't where we look.
pub fn find_app() -> Option<PathBuf> {
    pick_app(std::env::var(APP_ENV).ok().as_deref(), &app_candidates())
}

/// An override, if it points at something real, otherwise the first candidate
/// that exists. A set-but-wrong override resolves to nothing rather than
/// falling back, so a typo is visible instead of silently launching the app
/// the override meant to replace.
fn pick_app(override_path: Option<&str>, candidates: &[PathBuf]) -> Option<PathBuf> {
    if let Some(custom) = override_path.filter(|v| !v.is_empty()) {
        let path = PathBuf::from(custom);
        return path.exists().then_some(path);
    }
    candidates.iter().find(|p| p.exists()).cloned()
}

fn app_candidates() -> Vec<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        let mut paths = vec![PathBuf::from("/Applications/Claude.app")];
        if let Some(home) = dirs::home_dir() {
            paths.push(home.join("Applications/Claude.app"));
        }
        paths
    }
    #[cfg(not(target_os = "macos"))]
    {
        Vec::new()
    }
}

/// The command that opens `app` on `profile`.
///
/// `open -n` is what makes concurrent instances possible: it starts a new
/// process rather than activating the running one, and everything after
/// `--args` is handed to the app itself.
pub fn launch_command(app: &Path, profile: &Path) -> Command {
    let mut cmd = Command::new("open");
    cmd.arg("-n")
        .arg("-a")
        .arg(app)
        .arg("--args")
        .arg(format!("--user-data-dir={}", profile.display()));
    cmd
}

/// The app's MCP servers and preferences, inside the profile directory. New
/// profiles start without one.
pub const CONFIG_FILE: &str = "claude_desktop_config.json";

/// How the profile is shown in a message: managed profiles by name, the app's
/// own by the abbreviated path `list` already uses for it.
pub const STANDARD_LABEL: &str = "~/Library/…/Claude/";

#[derive(Debug, PartialEq)]
pub enum ClonePlan {
    /// Nothing to copy — the source has no config of its own.
    NoSource,
    /// The destination already has one; copying would discard it.
    Keep,
    Copy,
}

/// Whether to copy the config file. Split out from the copying so the
/// "don't silently discard what's already there" rule can be tested without
/// a filesystem.
pub fn plan_clone(source_exists: bool, dest_exists: bool, force: bool) -> ClonePlan {
    if !source_exists {
        return ClonePlan::NoSource;
    }
    if dest_exists && !force {
        return ClonePlan::Keep;
    }
    ClonePlan::Copy
}

/// Copy `CONFIG_FILE` from one profile into another, via a staging file so an
/// interrupted copy can't leave the destination holding half a config.
///
/// `fs::copy` carries the source's mode across, which matters here: the file
/// can hold MCP server credentials and is `0600` in the app's own profile.
pub fn clone_config(source: &Path, dest: &Path) -> io::Result<u64> {
    let src = source.join(CONFIG_FILE);
    let dst = dest.join(CONFIG_FILE);
    fs::create_dir_all(dest)?;
    let staged = dst.with_extension("json.part");
    let _ = fs::remove_file(&staged);
    let bytes = fs::copy(&src, &staged)?;
    fs::rename(&staged, &dst)?;
    Ok(bytes)
}

/// Whether the profile holds a signed-in session.
///
/// Only the presence of a credential is checked, not its validity — reading
/// the identity behind it needs the Keychain and is a separate step (#75).
/// `oauth:tokenCache` is the pre-V2 key and can be left behind as an empty
/// placeholder, hence the emptiness check on both.
pub fn is_signed_in(profile: &Path) -> bool {
    let Ok(raw) = fs::read_to_string(profile.join("config.json")) else {
        return false;
    };
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return false;
    };
    ["oauth:tokenCacheV2", "oauth:tokenCache"]
        .iter()
        .any(|key| {
            json.get(*key)
                .and_then(|v| v.as_str())
                .is_some_and(|v| !v.is_empty())
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_config(tag: &str) -> AppConfig {
        let base = std::env::temp_dir().join(format!("cc-desktop-{}-{}", tag, std::process::id()));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(&base).unwrap();
        AppConfig { base_dir: base }
    }

    #[test]
    fn profiles_live_next_to_accounts() {
        let c = temp_config("layout");
        assert_eq!(profiles_dir(&c), c.base_dir.join("desktop"));
        assert_eq!(profile_path(&c, "work"), c.base_dir.join("desktop/work"));
        let _ = fs::remove_dir_all(&c.base_dir);
    }

    #[test]
    fn listing_is_empty_when_nothing_was_ever_added() {
        // The desktop dir is created lazily, so a missing one is normal
        // rather than an error.
        let c = temp_config("empty");
        assert!(list_profiles(&c).unwrap().is_empty());
        let _ = fs::remove_dir_all(&c.base_dir);
    }

    #[test]
    fn listing_returns_directories_sorted_and_ignores_files() {
        let c = temp_config("list");
        fs::create_dir_all(profile_path(&c, "work")).unwrap();
        fs::create_dir_all(profile_path(&c, "personal")).unwrap();
        fs::write(profiles_dir(&c).join("stray.txt"), "").unwrap();
        assert_eq!(list_profiles(&c).unwrap(), vec!["personal", "work"]);
        let _ = fs::remove_dir_all(&c.base_dir);
    }

    fn write_config(profile: &Path, body: &str) {
        fs::create_dir_all(profile).unwrap();
        fs::write(profile.join("config.json"), body).unwrap();
    }

    #[test]
    fn a_fresh_profile_is_not_signed_in() {
        let c = temp_config("fresh");
        let p = profile_path(&c, "work");
        fs::create_dir_all(&p).unwrap();
        assert!(!is_signed_in(&p), "no config.json at all");
        write_config(&p, r#"{"locale":"en-US"}"#);
        assert!(!is_signed_in(&p), "config.json without a token");
        let _ = fs::remove_dir_all(&c.base_dir);
    }

    #[test]
    fn either_token_key_counts_as_signed_in() {
        let c = temp_config("token");
        let p = profile_path(&c, "work");
        write_config(&p, r#"{"oauth:tokenCacheV2":"djEw..."}"#);
        assert!(is_signed_in(&p), "V2 key");
        write_config(&p, r#"{"oauth:tokenCache":"djEw..."}"#);
        assert!(is_signed_in(&p), "pre-V2 key");
        let _ = fs::remove_dir_all(&c.base_dir);
    }

    #[test]
    fn an_empty_token_placeholder_is_not_signed_in() {
        // The V1 entry survives the V2 migration as an empty string.
        let c = temp_config("placeholder");
        let p = profile_path(&c, "work");
        write_config(&p, r#"{"oauth:tokenCache":"","locale":"en-US"}"#);
        assert!(!is_signed_in(&p));
        let _ = fs::remove_dir_all(&c.base_dir);
    }

    #[test]
    fn a_corrupt_config_is_not_signed_in() {
        let c = temp_config("corrupt");
        let p = profile_path(&c, "work");
        write_config(&p, "{not json");
        assert!(!is_signed_in(&p));
        let _ = fs::remove_dir_all(&c.base_dir);
    }

    #[test]
    fn launch_passes_the_profile_after_args() {
        // Order matters: anything before `--args` is consumed by `open`
        // itself, so a misplaced switch would silently do nothing.
        let cmd = launch_command(Path::new("/Applications/Claude.app"), Path::new("/tmp/p"));
        let args: Vec<String> = cmd
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert_eq!(cmd.get_program(), "open");
        assert_eq!(
            args,
            vec![
                "-n",
                "-a",
                "/Applications/Claude.app",
                "--args",
                "--user-data-dir=/tmp/p",
            ]
        );
    }

    #[test]
    fn a_profile_path_with_a_space_stays_one_argument() {
        // No shell is involved, so the path needs no quoting — and must not
        // get any, or the app would look for a directory named `"..."`.
        let cmd = launch_command(Path::new("/Applications/Claude.app"), Path::new("/tmp/a b"));
        let last = cmd
            .get_args()
            .last()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        assert_eq!(last, "--user-data-dir=/tmp/a b");
    }

    #[test]
    fn the_override_wins_over_the_usual_locations() {
        let c = temp_config("app");
        let real = c.base_dir.join("Claude.app");
        let other = c.base_dir.join("Other.app");
        fs::create_dir_all(&real).unwrap();
        fs::create_dir_all(&other).unwrap();

        let candidates = vec![other.clone()];
        assert_eq!(
            pick_app(Some(real.to_str().unwrap()), &candidates),
            Some(real)
        );
        assert_eq!(pick_app(None, &candidates), Some(other.clone()));
        assert_eq!(pick_app(Some(""), &candidates), Some(other));
        let _ = fs::remove_dir_all(&c.base_dir);
    }

    #[test]
    fn a_set_but_missing_override_finds_nothing() {
        // Falling back would launch the very app the override meant to
        // replace, and the typo would never surface.
        let c = temp_config("app-missing");
        let present = c.base_dir.join("Claude.app");
        fs::create_dir_all(&present).unwrap();
        assert_eq!(pick_app(Some("/no/such/app"), &[present]), None);
        let _ = fs::remove_dir_all(&c.base_dir);
    }

    #[test]
    fn cloning_needs_a_source_config() {
        assert_eq!(plan_clone(false, false, false), ClonePlan::NoSource);
        // Even --force can't copy a file that isn't there.
        assert_eq!(plan_clone(false, true, true), ClonePlan::NoSource);
    }

    #[test]
    fn an_existing_destination_config_is_kept_unless_forced() {
        // The file holds MCP servers someone configured by hand; silently
        // replacing it would be the worst possible default.
        assert_eq!(plan_clone(true, true, false), ClonePlan::Keep);
        assert_eq!(plan_clone(true, true, true), ClonePlan::Copy);
    }

    #[test]
    fn an_empty_destination_is_copied_into() {
        assert_eq!(plan_clone(true, false, false), ClonePlan::Copy);
    }

    #[test]
    fn clone_config_writes_the_file_and_leaves_no_staging_behind() {
        let c = temp_config("clone");
        let src = profile_path(&c, "source");
        let dst = profile_path(&c, "target");
        fs::create_dir_all(&src).unwrap();
        let body = r#"{"mcpServers":{"docker":{"command":"docker"}}}"#;
        fs::write(src.join(CONFIG_FILE), body).unwrap();

        let bytes = clone_config(&src, &dst).unwrap();
        assert_eq!(bytes as usize, body.len());
        assert_eq!(fs::read_to_string(dst.join(CONFIG_FILE)).unwrap(), body);
        assert!(
            !dst.join("claude_desktop_config.json.part").exists(),
            "staging file survived"
        );
        let _ = fs::remove_dir_all(&c.base_dir);
    }

    #[test]
    fn clone_config_replaces_rather_than_appends() {
        // Regression shape: a plain write over a longer existing file would
        // leave the old tail behind.
        let c = temp_config("clone-over");
        let src = profile_path(&c, "source");
        let dst = profile_path(&c, "target");
        fs::create_dir_all(&src).unwrap();
        fs::create_dir_all(&dst).unwrap();
        fs::write(src.join(CONFIG_FILE), "{}").unwrap();
        fs::write(dst.join(CONFIG_FILE), r#"{"mcpServers":{"old":{}}}"#).unwrap();

        clone_config(&src, &dst).unwrap();
        assert_eq!(fs::read_to_string(dst.join(CONFIG_FILE)).unwrap(), "{}");
        let _ = fs::remove_dir_all(&c.base_dir);
    }

    #[test]
    fn no_candidate_exists_means_no_app() {
        assert_eq!(pick_app(None, &[PathBuf::from("/no/such/app")]), None);
        assert_eq!(pick_app(None, &[]), None);
    }
}
