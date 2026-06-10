use std::path::Path;
use std::process::Command;

use crate::commands::install::binary_name;
use crate::config::AppConfig;
use crate::i18n::{I18n, Msg};

// Self-update: query the GitHub Releases of the repo this binary was built
// from, and if a newer tag is published, download the matching prebuilt asset
// and swap it in over `~/.claude-switch/bin/claude-acc`. Shells out to `curl`
// (already a dependency for the OAuth calls) to avoid pulling in HTTP/TLS deps.

const REPO_URL: &str = env!("CARGO_PKG_REPOSITORY");
const CURRENT: &str = env!("CARGO_PKG_VERSION");

pub fn run(config: &AppConfig, i18n: &I18n, check_only: bool) -> i32 {
    let Some(slug) = repo_slug(REPO_URL) else {
        i18n.print(Msg::UpdateRepoUnknown);
        return 1;
    };

    let Some(tag) = latest_release_tag(&slug) else {
        i18n.print(Msg::UpdateCheckFailed);
        return 1;
    };
    let latest = tag.trim_start_matches('v');

    if !is_newer(latest, CURRENT) {
        i18n.print(Msg::UpdateUpToDate(CURRENT.to_string()));
        return 0;
    }

    i18n.print(Msg::UpdateAvailable(
        CURRENT.to_string(),
        latest.to_string(),
    ));
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
    i18n.print(Msg::UpdateDownloading(latest.to_string()));
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

    i18n.print(Msg::UpdateDone(
        latest.to_string(),
        target.display().to_string(),
    ));
    0
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

fn latest_release_tag(slug: &str) -> Option<String> {
    let url = format!("https://api.github.com/repos/{}/releases/latest", slug);
    let out = Command::new("curl")
        .args(["-fsSL", "--max-time", "10"])
        .args(["-H", "User-Agent: claude-acc"])
        .args(["-H", "Accept: application/vnd.github+json"])
        .arg(&url)
        .output()
        .ok()?;
    if !out.status.success() || out.stdout.is_empty() {
        return None;
    }
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).ok()?;
    v.get("tag_name")?.as_str().map(String::from)
}

/// "owner/repo" from a GitHub URL (https or scp-style git@), else `None`.
fn repo_slug(url: &str) -> Option<String> {
    let s = url.trim().trim_end_matches('/');
    let s = s.strip_suffix(".git").unwrap_or(s);
    let rest = s
        .strip_prefix("https://github.com/")
        .or_else(|| s.strip_prefix("http://github.com/"))
        .or_else(|| s.strip_prefix("git@github.com:"))?;
    let parts: Vec<&str> = rest.split('/').collect();
    if parts.len() == 2 && !parts[0].is_empty() && !parts[1].is_empty() {
        Some(format!("{}/{}", parts[0], parts[1]))
    } else {
        None
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

fn is_newer(latest: &str, current: &str) -> bool {
    match (parse_version(latest), parse_version(current)) {
        (Some(l), Some(c)) => l > c,
        _ => false,
    }
}

/// Parse "X.Y.Z" into a comparable tuple. Trailing pre-release/build metadata
/// on the patch (e.g. "0-rc1") is ignored — we only ship clean tags.
fn parse_version(v: &str) -> Option<(u32, u32, u32)> {
    let mut it = v.trim().split('.');
    let major = it.next()?.parse().ok()?;
    let minor = it.next()?.parse().ok()?;
    let patch_field = it.next()?;
    let patch_digits: String = patch_field
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    let patch = patch_digits.parse().ok()?;
    Some((major, minor, patch))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repo_slug_https() {
        assert_eq!(
            repo_slug("https://github.com/Nemo-Illusionist/claude-code-account-switcher"),
            Some("Nemo-Illusionist/claude-code-account-switcher".to_string())
        );
    }

    #[test]
    fn repo_slug_strips_git_suffix_and_trailing_slash() {
        assert_eq!(
            repo_slug("https://github.com/owner/repo.git"),
            Some("owner/repo".to_string())
        );
        assert_eq!(
            repo_slug("https://github.com/owner/repo/"),
            Some("owner/repo".to_string())
        );
    }

    #[test]
    fn repo_slug_scp_style() {
        assert_eq!(
            repo_slug("git@github.com:owner/repo.git"),
            Some("owner/repo".to_string())
        );
    }

    #[test]
    fn repo_slug_rejects_non_github_or_malformed() {
        assert_eq!(repo_slug("https://gitlab.com/owner/repo"), None);
        assert_eq!(repo_slug("https://github.com/owner"), None);
        assert_eq!(repo_slug("https://github.com/owner/repo/extra"), None);
    }

    #[test]
    fn parse_version_basic() {
        assert_eq!(parse_version("0.8.0"), Some((0, 8, 0)));
        assert_eq!(parse_version("12.3.45"), Some((12, 3, 45)));
        assert_eq!(parse_version("1.2.3-rc1"), Some((1, 2, 3)));
    }

    #[test]
    fn parse_version_rejects_garbage() {
        assert_eq!(parse_version("not.a.version"), None);
        assert_eq!(parse_version("1.2"), None);
    }

    #[test]
    fn is_newer_compares_correctly() {
        assert!(is_newer("0.9.0", "0.8.0"));
        assert!(is_newer("0.8.1", "0.8.0"));
        assert!(is_newer("1.0.0", "0.99.99"));
        assert!(!is_newer("0.8.0", "0.8.0"));
        assert!(!is_newer("0.7.9", "0.8.0"));
        // Unparseable input is treated as "not newer" (fail safe).
        assert!(!is_newer("garbage", "0.8.0"));
    }
}
