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
use std::path::Path;
use std::process::Command;

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
    let Some(token) = read_token(acc_dir) else {
        return AuditResult::NoToken;
    };
    match fetch_profile(&token) {
        Some(p) => AuditResult::Ok(p),
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
