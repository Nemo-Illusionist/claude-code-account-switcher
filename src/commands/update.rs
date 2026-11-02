use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::commands::install::binary_name;
use crate::config::AppConfig;
use crate::i18n::{I18n, Msg};
use crate::update::{
    CURRENT, REPO_URL, is_newer, latest_release_tag, normalize_version, repo_slug,
};

// Self-update: query the GitHub Releases of the repo this binary was built
// from, and if a newer tag is published, download the matching prebuilt asset
// and swap it in over `~/.claude-switch/bin/claude-acc`. Shells out to `curl`
// (already a dependency for the OAuth calls) to avoid pulling in HTTP/TLS deps.

/// How often `maybe_print_hint` is willing to hit the network — once a day,
/// so the passive hint printed after ordinary commands never adds real
/// latency to the common case.
const HINT_CHECK_INTERVAL_SECS: u64 = 24 * 60 * 60;

pub fn run(config: &AppConfig, i18n: &I18n, check_only: bool, version: Option<&str>) -> i32 {
    let Some(slug) = repo_slug(REPO_URL) else {
        i18n.print(Msg::UpdateRepoUnknown);
        return 1;
    };

    // Pinning to an explicit --version skips the "is this actually newer"
    // check entirely (below) — that check exists to avoid noisy re-downloads
    // on every `update` run, but it would also block a deliberate rollback to
    // an older release, which is the whole point of --version.
    let (tag, target_version) = match version {
        Some(v) => {
            let Some(target_version) = normalize_version(v) else {
                i18n.print(Msg::UpdateInvalidVersion(v.to_string()));
                return 1;
            };
            let tag = format!("v{target_version}");
            if !release_tag_exists(&slug, &tag) {
                i18n.print(Msg::UpdateVersionNotFound(target_version));
                return 1;
            }
            (tag, target_version)
        }
        None => {
            let Some(tag) = latest_release_tag(&slug) else {
                i18n.print(Msg::UpdateCheckFailed);
                return 1;
            };
            let target_version = tag.trim_start_matches('v').to_string();
            (tag, target_version)
        }
    };

    if target_version == CURRENT {
        i18n.print(Msg::UpdateUpToDate(CURRENT.to_string()));
        return 0;
    }
    if version.is_none() && !is_newer(&target_version, CURRENT) {
        i18n.print(Msg::UpdateUpToDate(CURRENT.to_string()));
        return 0;
    }

    if is_newer(&target_version, CURRENT) {
        i18n.print(Msg::UpdateAvailable(
            CURRENT.to_string(),
            target_version.clone(),
        ));
    } else {
        // Only reachable via --version pinning to an older release — the
        // "is this newer" check above already filtered this out otherwise.
        i18n.print(Msg::UpdateDowngrading(
            CURRENT.to_string(),
            target_version.clone(),
        ));
    }
    if check_only {
        return 0;
    }

    let Some(asset) = asset_name() else {
        i18n.print(Msg::UpdateUnsupportedPlatform);
        return 1;
    };

    let bin_dir = config.base_dir.join("bin");
    if std::fs::create_dir_all(&bin_dir).is_err() {
        i18n.print(Msg::UpdateReplaceFailed(bin_dir.display().to_string()));
        return 1;
    }
    let target = bin_dir.join(binary_name());
    let tmp = bin_dir.join(format!("{}.new", binary_name()));

    let url = format!(
        "https://github.com/{}/releases/download/{}/{}",
        slug, tag, asset
    );
    i18n.print(Msg::UpdateDownloading(target_version.clone()));
    if !download(&url, &tmp) {
        let _ = std::fs::remove_file(&tmp);
        i18n.print(Msg::UpdateDownloadFailed);
        return 1;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755));
    }

    if let Err(e) = replace_binary(&tmp, &target) {
        let _ = std::fs::remove_file(&tmp);
        i18n.print(Msg::UpdateReplaceFailed(e.to_string()));
        return 1;
    }

    refresh_wrapper(config, &target, i18n);

    i18n.print(Msg::UpdateDone(
        target_version,
        target.display().to_string(),
    ));
    0
}

/// The generated `claude` wrapper embeds the path to this binary *and*
/// carries behaviour of its own — the `--resume` preflight, for one. A
/// binary-only update would leave a stale script sitting in front of every
/// `claude` launch, silently missing whatever the new version added, with
/// nothing to hint at why.
///
/// Only refreshes a wrapper that is already there: an update is not the
/// moment to start installing shell integration behind someone's back, and
/// on Windows there is no wrapper to install at all.
fn refresh_wrapper(config: &AppConfig, binary: &Path, i18n: &I18n) {
    if !wrapper_path(config).exists() {
        return;
    }
    match crate::ide::install_wrapper(config, binary) {
        Ok(_) => i18n.print(Msg::UpdateWrapperRefreshed),
        // Non-fatal: the new binary is already in place and works. Say so
        // rather than failing an otherwise successful update.
        Err(e) => i18n.print(Msg::UpdateWrapperFailed(e.to_string())),
    }
    refresh_vscode_wrapper(config, binary, i18n);
}

/// Same reasoning for the VS Code launcher, and same rule: refresh one that
/// is already there, never install one that isn't.
#[cfg(not(windows))]
fn refresh_vscode_wrapper(config: &AppConfig, binary: &Path, i18n: &I18n) {
    if !crate::vscode::wrapper_path(&config.base_dir).exists() {
        return;
    }
    if let Err(e) = crate::vscode::install_wrapper(&config.base_dir, binary) {
        i18n.print(Msg::UpdateWrapperFailed(e.to_string()));
    }
}

#[cfg(windows)]
fn refresh_vscode_wrapper(_config: &AppConfig, _binary: &Path, _i18n: &I18n) {}

/// Where `install` puts the generated `claude` wrapper.
fn wrapper_path(config: &AppConfig) -> PathBuf {
    config.base_dir.join("bin").join("claude")
}

/// Replace `target` with the freshly-downloaded `tmp` (same directory, so the
/// rename is atomic on the same filesystem).
#[cfg(unix)]
fn replace_binary(tmp: &Path, target: &Path) -> std::io::Result<()> {
    // On Unix the running process keeps its open inode, so renaming a new file
    // over the path it was launched from is safe.
    std::fs::rename(tmp, target)
}

/// Windows can't overwrite a running `.exe`, so move the old one aside first.
#[cfg(windows)]
fn replace_binary(tmp: &Path, target: &Path) -> std::io::Result<()> {
    if target.exists() {
        let old = target.with_extension("old");
        let _ = std::fs::remove_file(&old);
        std::fs::rename(target, &old)?;
    }
    std::fs::rename(tmp, target)
}

fn download(url: &str, dest: &Path) -> bool {
    Command::new("curl")
        .args(["-fsSL", "--retry", "2", "--max-time", "300"])
        .args(["-H", "User-Agent: claude-acc"])
        .arg("-o")
        .arg(dest)
        .arg(url)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Whether GitHub has a published release for `tag` (e.g. "v0.10.5"). Used
/// to give a clear "no such version" error for `--version` instead of
/// letting a bogus version fall through to a confusing download failure.
fn release_tag_exists(slug: &str, tag: &str) -> bool {
    let url = format!(
        "https://api.github.com/repos/{}/releases/tags/{}",
        slug, tag
    );
    Command::new("curl")
        .args(["-fsSL", "--max-time", "10"])
        .args(["-H", "User-Agent: claude-acc"])
        .args(["-H", "Accept: application/vnd.github+json"])
        .arg(&url)
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

fn hint_cache_path(config: &AppConfig) -> PathBuf {
    config.base_dir.join("update-check.json")
}

/// `latest: None` means the last attempt failed (network down, API error) —
/// still recorded, so a persistent outage doesn't turn into a GitHub API
/// call on every single command for as long as it lasts.
struct HintCache {
    checked_at: u64,
    latest: Option<String>,
}

fn read_hint_cache(path: &Path) -> Option<HintCache> {
    let content = std::fs::read_to_string(path).ok()?;
    let v: serde_json::Value = serde_json::from_str(&content).ok()?;
    Some(HintCache {
        checked_at: v.get("checked_at").and_then(|x| x.as_u64())?,
        latest: v.get("latest").and_then(|x| x.as_str()).map(String::from),
    })
}

fn write_hint_cache(path: &Path, cache: &HintCache) {
    let body = serde_json::json!({
        "checked_at": cache.checked_at,
        "latest": cache.latest,
    });
    if let Ok(serialized) = serde_json::to_string(&body) {
        let _ = std::fs::write(path, serialized);
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Passive "a new version is out" hint, printed after most commands finish.
/// Checks GitHub Releases at most once every 24h (cached in
/// `~/.claude-switch/update-check.json`); every other invocation reuses the
/// cached answer with no network call at all. Silent on any failure — this
/// is a courtesy nudge, not something that should ever block, slow down, or
/// error out a command a user is actually trying to run.
pub fn maybe_print_hint(config: &AppConfig, i18n: &I18n) {
    let path = hint_cache_path(config);
    let cached = read_hint_cache(&path);
    let now = now_secs();

    let latest = match &cached {
        Some(c) if now.saturating_sub(c.checked_at) < HINT_CHECK_INTERVAL_SECS => c.latest.clone(),
        _ => {
            let Some(slug) = repo_slug(REPO_URL) else {
                return;
            };
            let latest = latest_release_tag(&slug).map(|t| t.trim_start_matches('v').to_string());
            write_hint_cache(
                &path,
                &HintCache {
                    checked_at: now,
                    latest: latest.clone(),
                },
            );
            latest
        }
    };

    if let Some(latest) = latest
        && is_newer(&latest, CURRENT)
    {
        i18n.print(Msg::UpdateHintAvailable(CURRENT.to_string(), latest));
    }
}

/// The release asset name matching the host OS + architecture, as produced by
/// the release workflow. `None` on platforms we don't publish binaries for.
fn asset_name() -> Option<&'static str> {
    let arch = std::env::consts::ARCH;
    if cfg!(target_os = "macos") {
        match arch {
            "x86_64" => Some("claude-acc-macos-x86_64"),
            "aarch64" => Some("claude-acc-macos-aarch64"),
            _ => None,
        }
    } else if cfg!(target_os = "linux") {
        match arch {
            "x86_64" => Some("claude-acc-linux-x86_64"),
            "aarch64" => Some("claude-acc-linux-aarch64"),
            _ => None,
        }
    } else if cfg!(target_os = "windows") {
        match arch {
            "x86_64" => Some("claude-acc-windows-x86_64.exe"),
            _ => None,
        }
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hint_cache_roundtrips_with_a_version() {
        let path = std::env::temp_dir().join(format!(
            "claude-acc-test-hint-cache-with-version-{}.json",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);

        write_hint_cache(
            &path,
            &HintCache {
                checked_at: 12345,
                latest: Some("0.11.5".to_string()),
            },
        );
        let read = read_hint_cache(&path).expect("should read back what was written");

        let _ = std::fs::remove_file(&path);
        assert_eq!(read.checked_at, 12345);
        assert_eq!(read.latest, Some("0.11.5".to_string()));
    }

    #[test]
    fn hint_cache_roundtrips_a_failed_check() {
        let path = std::env::temp_dir().join(format!(
            "claude-acc-test-hint-cache-failed-{}.json",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);

        write_hint_cache(
            &path,
            &HintCache {
                checked_at: 999,
                latest: None,
            },
        );
        let read = read_hint_cache(&path).expect("should read back what was written");

        let _ = std::fs::remove_file(&path);
        assert_eq!(read.checked_at, 999);
        assert_eq!(read.latest, None);
    }

    fn temp_config(tag: &str) -> AppConfig {
        let base = std::env::temp_dir().join(format!("cc-update-{}-{}", tag, std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let config = AppConfig { base_dir: base };
        config.init().unwrap();
        config
    }

    #[test]
    fn wrapper_path_matches_where_install_writes_it() {
        let config = temp_config("wrapper-path");
        assert_eq!(wrapper_path(&config), config.base_dir.join("bin/claude"));
        let _ = std::fs::remove_dir_all(&config.base_dir);
    }

    #[test]
    fn refresh_does_not_create_a_wrapper_that_was_never_installed() {
        // An update must not start adding shell integration on its own —
        // and on Windows there is no wrapper to write in the first place.
        let config = temp_config("no-wrapper");
        let i18n = I18n {
            lang: crate::i18n::Lang::En,
        };
        refresh_wrapper(&config, &config.base_dir.join("claude-acc"), &i18n);
        assert!(!wrapper_path(&config).exists());
        let _ = std::fs::remove_dir_all(&config.base_dir);
    }

    #[cfg(unix)]
    #[test]
    fn refresh_rewrites_an_existing_wrapper_with_the_new_binary_path() {
        let config = temp_config("stale-wrapper");
        let i18n = I18n {
            lang: crate::i18n::Lang::En,
        };
        let wrapper = wrapper_path(&config);
        std::fs::create_dir_all(wrapper.parent().unwrap()).unwrap();
        std::fs::write(
            &wrapper,
            "#!/bin/sh\n# stale wrapper from an older version\n",
        )
        .unwrap();

        let binary = config.base_dir.join("claude-acc");
        refresh_wrapper(&config, &binary, &i18n);

        let content = std::fs::read_to_string(&wrapper).unwrap();
        assert!(!content.contains("stale wrapper"), "{}", content);
        assert!(
            content.contains(&binary.display().to_string()),
            "wrapper does not point at the new binary: {}",
            content
        );
        let _ = std::fs::remove_dir_all(&config.base_dir);
    }

    #[test]
    fn read_hint_cache_missing_file_returns_none() {
        let path = std::path::PathBuf::from("/definitely/does/not/exist/update-check.json");
        assert!(read_hint_cache(&path).is_none());
    }
}
