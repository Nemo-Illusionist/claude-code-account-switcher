//! Which `claude-acc` release is the latest, and how versions compare. The
//! CLI's `update` and the manager's update check share it.

use std::process::Command;

pub const REPO_URL: &str = env!("CARGO_PKG_REPOSITORY");
/// The version this library, and the binary built with it, comes from.
pub const CURRENT: &str = env!("CARGO_PKG_VERSION");

/// "owner/repo" from a GitHub URL (https or scp-style git@), else `None`.
pub fn repo_slug(url: &str) -> Option<String> {
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

pub fn latest_release_tag(slug: &str) -> Option<String> {
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

/// Normalizes a user-supplied `--version` value ("v0.10.5" or "0.10.5") to
/// the bare "X.Y.Z" form release tags use, validating it parses as a
/// version at all (catches typos before making any network call).
pub fn normalize_version(v: &str) -> Option<String> {
    let trimmed = v.trim().trim_start_matches('v');
    parse_version(trimmed)?;
    Some(trimmed.to_string())
}

pub fn is_newer(latest: &str, current: &str) -> bool {
    match (parse_version(latest), parse_version(current)) {
        (Some(l), Some(c)) => l > c,
        _ => false,
    }
}

/// Parse "X.Y.Z" into a comparable tuple. Trailing pre-release/build metadata
/// on the patch (e.g. "0-rc1") is ignored — we only ship clean tags.
pub fn parse_version(v: &str) -> Option<(u32, u32, u32)> {
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

    #[test]
    fn normalize_version_strips_leading_v() {
        assert_eq!(normalize_version("v0.10.5"), Some("0.10.5".to_string()));
        assert_eq!(normalize_version("0.10.5"), Some("0.10.5".to_string()));
    }

    #[test]
    fn normalize_version_trims_whitespace() {
        assert_eq!(normalize_version("  v0.10.5  "), Some("0.10.5".to_string()));
    }

    #[test]
    fn normalize_version_rejects_garbage() {
        assert_eq!(normalize_version("not-a-version"), None);
        assert_eq!(normalize_version("1.2"), None);
        assert_eq!(normalize_version(""), None);
    }
}
