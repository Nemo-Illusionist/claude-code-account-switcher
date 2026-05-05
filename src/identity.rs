// Read OAuth identity for a given account dir.
//
// Claude Code stores the OAuth token in macOS Keychain under service
// "Claude Code-credentials-<hash>" where hash = sha256(CLAUDE_CONFIG_DIR)
// truncated to 8 hex chars. This is reverse-engineered from Claude Code's
// internal `dV()` function and could change in future versions — if it does,
// keychain reads will silently miss and we'll fall back to .credentials.json.
//
// The plaintext .credentials.json fallback is what older Claude Code versions
// (and current Linux/Windows builds) use when no keychain backend is available.
//
// HTTP and shell-out (security, curl) instead of native deps to keep the
// binary small. The Anthropic OAuth profile endpoint is undocumented and
// might change.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use sha2::{Digest, Sha256};

pub struct Profile {
    pub email: Option<String>,
    pub uuid: Option<String>,
    #[allow(dead_code)]
    pub organization: Option<String>,
}

pub enum AuditResult {
    Ok(Profile),
    NoToken,
    Offline,
}

pub fn audit_account(acc_dir: &Path) -> AuditResult {
    let cache = acc_dir.join(".account-info.json");
    audit_at(acc_dir, &cache)
}

/// Audit the standard `~/.claude/` config dir — the un-managed identity that
/// claude falls back to when no link / configured default applies. Cache lives
/// inside our switch dir (`default.account-info.json`), never inside
/// `~/.claude/` itself.
pub fn audit_default(switch_dir: &Path) -> AuditResult {
    let Some(claude_dir) = standard_token_dir() else {
        return AuditResult::NoToken;
    };
    audit_at(&claude_dir, &default_cache_path(switch_dir))
}

pub fn standard_token_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".claude"))
}

pub fn default_cache_path(switch_dir: &Path) -> PathBuf {
    switch_dir.join("default.account-info.json")
}

fn audit_at(token_dir: &Path, cache_path: &Path) -> AuditResult {
    let Some(token) = read_token(token_dir) else {
        return AuditResult::NoToken;
    };
    match fetch_profile(&token) {
        Some(p) => {
            // Side effect: refresh the cache so list/status can show the
            // identity without re-hitting the API. Errors here are silent —
            // doctor's own output is the source of truth for this run.
            let _ = write_cache_at(cache_path, &p, &token);
            AuditResult::Ok(p)
        }
        None => AuditResult::Offline,
    }
}

fn read_token(acc_dir: &Path) -> Option<String> {
    if let Some(t) = keychain_token(acc_dir) {
        return Some(t);
    }
    plaintext_token(acc_dir)
}

fn keychain_token(acc_dir: &Path) -> Option<String> {
    if !cfg!(target_os = "macos") {
        return None;
    }
    let acc_str = acc_dir.to_str()?;
    let mut hasher = Sha256::new();
    hasher.update(acc_str.as_bytes());
    let digest = hasher.finalize();
    let hash: String = digest
        .iter()
        .take(4)
        .map(|b| format!("{:02x}", b))
        .collect();
    let service = format!("Claude Code-credentials-{}", hash);

    let user = whoami_short()?;
    let out = Command::new("security")
        .args(["find-generic-password", "-s", &service, "-a", &user, "-w"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let raw = String::from_utf8_lossy(&out.stdout).trim().to_string();
    extract_access_token(&raw)
}

fn plaintext_token(acc_dir: &Path) -> Option<String> {
    let path = acc_dir.join(".credentials.json");
    let content = fs::read_to_string(&path).ok()?;
    extract_access_token(&content)
}

fn extract_access_token(raw: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(raw).ok()?;
    v.get("claudeAiOauth")?
        .get("accessToken")?
        .as_str()
        .map(|s| s.to_string())
}

// --- Cache (.account-info.json) ---
//
// Written by `doctor` on every successful API audit. Read by `list` and
// `status` so they can show the email / fetched_at without hitting the API.
//
// `token_hash` is sha256(access_token) truncated to 16 hex chars. Used as a
// soft signal: if the current keychain token's hash matches what we cached,
// the cache is "stable since last verify". If it differs, the token has
// rotated since cache write — which is most often a routine OAuth refresh
// (identity unchanged) but could also be a re-auth to a different account.
// Callers display a `*` marker on mismatch and let the user decide whether
// to re-run `doctor`.

pub struct CachedInfo {
    pub email: Option<String>,
    #[allow(dead_code)] // serialized for doctor's stable-uuid comparison in future phases
    pub uuid: Option<String>,
    #[allow(dead_code)]
    pub org: Option<String>,
    pub fetched_at: Option<u64>,
    pub token_hash: Option<String>,
}

pub fn read_cache(acc_dir: &Path) -> Option<CachedInfo> {
    read_cache_at(&acc_dir.join(".account-info.json"))
}

pub fn read_cache_at(path: &Path) -> Option<CachedInfo> {
    let content = fs::read_to_string(path).ok()?;
    let v: serde_json::Value = serde_json::from_str(&content).ok()?;
    Some(CachedInfo {
        email: v.get("email").and_then(|x| x.as_str()).map(String::from),
        uuid: v.get("uuid").and_then(|x| x.as_str()).map(String::from),
        org: v.get("org").and_then(|x| x.as_str()).map(String::from),
        fetched_at: v.get("fetched_at").and_then(|x| x.as_u64()),
        token_hash: v
            .get("token_hash")
            .and_then(|x| x.as_str())
            .map(String::from),
    })
}

fn write_cache_at(path: &Path, profile: &Profile, token: &str) -> std::io::Result<()> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let body = serde_json::json!({
        "email": profile.email,
        "uuid": profile.uuid,
        "org": profile.organization,
        "fetched_at": now,
        "token_hash": token_hash(token),
    });
    let serialized = serde_json::to_string_pretty(&body).map_err(std::io::Error::other)?;
    fs::write(path, serialized)
}

pub fn token_hash(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    let digest = hasher.finalize();
    digest
        .iter()
        .take(8)
        .map(|b| format!("{:02x}", b))
        .collect()
}

/// Hash of the *current* keychain token for `token_dir`, for comparing
/// against `CachedInfo::token_hash`. `token_dir` is either an account dir
/// under `~/.claude-switch/accounts/` or `~/.claude/` for the standard
/// fallback. Returns `None` if no token (not on macOS, not logged in, or
/// `security` failed) — caller treats `None` as "can't verify, skip marker".
pub fn current_token_hash(token_dir: &Path) -> Option<String> {
    read_token(token_dir).map(|t| token_hash(&t))
}

/// Seconds elapsed since `fetched_at`. Returns `None` if the timestamp looks
/// invalid (in the future, or epoch).
pub fn seconds_since(fetched_at: u64) -> Option<u64> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs();
    if fetched_at == 0 || fetched_at > now {
        return None;
    }
    Some(now - fetched_at)
}

fn whoami_short() -> Option<String> {
    let out = Command::new("id").arg("-un").output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn fetch_profile(token: &str) -> Option<Profile> {
    // Shell out to curl to avoid pulling in HTTP+TLS deps. macOS ships curl;
    // Linux distros that have Claude Code installed also have curl.
    let out = Command::new("curl")
        .args(["-sf", "--max-time", "5"])
        .args(["-H", &format!("Authorization: Bearer {}", token)])
        .args(["-H", "anthropic-beta: oauth-2025-04-20"])
        .arg("https://api.anthropic.com/api/oauth/profile")
        .output()
        .ok()?;
    if !out.status.success() || out.stdout.is_empty() {
        return None;
    }
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).ok()?;
    Some(Profile {
        email: v
            .get("account")
            .and_then(|a| a.get("email"))
            .and_then(|e| e.as_str())
            .map(|s| s.to_string()),
        uuid: v
            .get("account")
            .and_then(|a| a.get("uuid"))
            .and_then(|u| u.as_str())
            .map(|s| s.to_string()),
        organization: v
            .get("organization")
            .and_then(|o| o.get("name"))
            .and_then(|n| n.as_str())
            .map(|s| s.to_string()),
    })
}
