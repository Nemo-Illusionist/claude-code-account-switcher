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

/// The pre-2.1 unscoped Keychain service name. Claude Code 2.1+ scopes
/// credentials per config dir (see `keychain_service`), but has been
/// observed to keep this bare entry in sync for the standard `~/.claude`
/// account too — `read_token` falls back to it when the scoped lookup for
/// the standard account misses. See its call site for why.
const LEGACY_KEYCHAIN_SERVICE: &str = "Claude Code-credentials";

pub struct Profile {
    pub email: Option<String>,
    pub uuid: Option<String>,
    #[allow(dead_code)]
    pub organization: Option<String>,
    /// Friendly subscription label, e.g. "Max 20x" / "Pro". `None` when the
    /// profile carries no recognizable plan.
    pub plan: Option<String>,
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

/// Where Claude Code keeps its own record of the signed-in account for a
/// config dir.
///
/// It writes `.claude.json` next to — not inside — the config dir it was
/// given: `join(CLAUDE_CONFIG_DIR ?? homedir(), ".claude.json")`. So a
/// managed account has it inside the account directory, and the standard
/// account has it at `~/.claude.json`, beside `~/.claude/` rather than in it.
pub fn local_identity_path(config_dir: &Path) -> Option<PathBuf> {
    if Some(config_dir.to_path_buf()) == standard_token_dir() {
        return dirs::home_dir().map(|h| h.join(".claude.json"));
    }
    Some(config_dir.join(".claude.json"))
}

/// The account Claude Code last signed this config dir in as, read from its
/// own file.
///
/// This is the cheap way to answer "who is this?": a local JSON read, no
/// keychain prompt and no network, which is what makes it usable somewhere
/// that runs on every launch. It is Claude Code's cache rather than the
/// authority — but drift happens *through* a login, and a login is exactly
/// what rewrites this file.
pub fn local_identity(config_dir: &Path) -> Option<Identity> {
    let path = local_identity_path(config_dir)?;
    let raw = fs::read_to_string(path).ok()?;
    let v: serde_json::Value = serde_json::from_str(&raw).ok()?;
    let account = v.get("oauthAccount")?;
    Some(Identity {
        uuid: account
            .get("accountUuid")
            .and_then(|x| x.as_str())
            .map(String::from)?,
        email: account
            .get("emailAddress")
            .and_then(|x| x.as_str())
            .map(String::from),
    })
}

/// The account an account dir is pinned to, or is currently signed in as.
#[derive(Clone, Debug, PartialEq)]
pub struct Identity {
    pub uuid: String,
    pub email: Option<String>,
}

/// Filename of the pin, inside the account dir.
pub const LOCK_FILE: &str = ".identity-lock.json";

/// Where the pin for the standard `~/.claude/` account lives.
///
/// Beside our own state, not inside `~/.claude/` — the same rule the doctor
/// cache follows. That directory belongs to Claude Code; we read it and do
/// not litter it.
pub fn default_lock_path(switch_dir: &Path) -> PathBuf {
    switch_dir.join("default.identity-lock.json")
}

/// Read the pin at `path`. Callers pass the path rather than the account dir
/// because the standard account keeps its pin outside `~/.claude/`.
pub fn read_lock_at(path: &Path) -> Option<Identity> {
    let raw = fs::read_to_string(path).ok()?;
    let v: serde_json::Value = serde_json::from_str(&raw).ok()?;
    Some(Identity {
        uuid: v.get("uuid").and_then(|x| x.as_str()).map(String::from)?,
        email: v.get("email").and_then(|x| x.as_str()).map(String::from),
    })
}

/// Pin `path` to `identity`.
pub fn write_lock_at(path: &Path, identity: &Identity) -> std::io::Result<()> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let body = serde_json::json!({
        "uuid": identity.uuid,
        "email": identity.email,
        "locked_at": now,
    });
    let serialized = serde_json::to_string_pretty(&body).map_err(std::io::Error::other)?;
    fs::write(path, serialized)
}

/// What the pin says about an account dir right now.
#[derive(Debug, PartialEq)]
pub enum LockState {
    /// Pinned, and the account signed in matches.
    Ok,
    /// Pinned to one account, signed in as another. The thing this exists to
    /// catch: a re-login that quietly swapped identity underneath a directory
    /// you had already decided belongs to someone.
    Drift {
        expected: Identity,
        actual: Identity,
    },
    /// No pin — nothing to compare against.
    NoLock,
    /// Pinned, but Claude Code has not recorded who is signed in. A directory
    /// that has never been logged in looks like this.
    Unknown,
}

/// Value-in, value-out so the comparison can be checked without a filesystem.
///
/// The uuid decides. An email can change on the same account, and two
/// accounts can share a display name; the uuid is the only stable identifier
/// here, and comparing on anything softer would produce drift reports that
/// are wrong in both directions.
pub fn compare_lock(lock: Option<&Identity>, current: Option<&Identity>) -> LockState {
    match (lock, current) {
        (None, _) => LockState::NoLock,
        (Some(_), None) => LockState::Unknown,
        (Some(l), Some(c)) if l.uuid == c.uuid => LockState::Ok,
        (Some(l), Some(c)) => LockState::Drift {
            expected: l.clone(),
            actual: c.clone(),
        },
    }
}

/// The pin state of an account dir, read from disk. `lock_path` is given
/// separately so the standard account can keep its pin outside `~/.claude/`.
pub fn lock_state_at(lock_path: &Path, config_dir: &Path) -> LockState {
    compare_lock(
        read_lock_at(lock_path).as_ref(),
        local_identity(config_dir).as_ref(),
    )
}

/// Does `(uuid, email)` identify the same account as `cached`? Uuid is the
/// stable signal and takes priority; email is compared case-insensitively as
/// a fallback for a `cached` entry that predates uuid caching. Pure — no I/O.
fn identity_matches(uuid: Option<&str>, email: Option<&str>, cached: &CachedInfo) -> bool {
    let uuid_match = uuid.is_some() && uuid == cached.uuid.as_deref();
    if uuid_match {
        return true;
    }
    email.is_some()
        && email.map(str::to_lowercase) == cached.email.as_deref().map(str::to_lowercase)
}

/// Best-effort duplicate-account hint: audits `new_dir`'s live identity (also
/// refreshing its cache, as a side effect of `audit_account`) and compares it
/// against the *cached* identity of every account in `known` — each
/// `(label, cache_path)` pair, caller-supplied so it can point at either a
/// managed account's `.account-info.json` or the standard account's
/// `default.account-info.json` under whatever label it wants shown (e.g.
/// "~/.claude/"). Caller must exclude `new_dir` itself from `known`.
///
/// Returns the label of the first match, or `None` if `new_dir` can't be
/// freshly audited (offline / no token) or nothing matches. "Best-effort"
/// because it only sees accounts a prior `doctor` run cached — one never
/// audited won't be caught. Mirrors github.com/stablyai/orca's
/// findDuplicateClaudeAccount, adapted to our CLI's synchronous, cache-based
/// comparison (no daemon keeping identities warm).
pub fn find_duplicate_account(new_dir: &Path, known: &[(String, PathBuf)]) -> Option<String> {
    let AuditResult::Ok(profile) = audit_account(new_dir) else {
        return None;
    };
    known.iter().find_map(|(label, cache_path)| {
        let cached = read_cache_at(cache_path)?;
        identity_matches(profile.uuid.as_deref(), profile.email.as_deref(), &cached)
            .then(|| label.clone())
    })
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
    // Fall back to the bare legacy service, but only for the standard
    // account: its scoped hash has been observed to drift from our own
    // sha256(CLAUDE_CONFIG_DIR)[0:8] computation (e.g. right after a fresh
    // `claude auth login`), silently breaking the lookup above even though
    // the account is genuinely logged in. The legacy entry stays in sync
    // for the standard account regardless. Not applied to managed accounts:
    // if a managed account's own scoped entry ever goes missing, falling
    // back to this shared/legacy entry could silently attribute a
    // *different* account's identity to it — worse than reporting no token.
    if should_try_legacy_keychain_fallback(acc_dir)
        && let Some(user) = whoami_short()
        && let Some(t) = read_keychain_blob(LEGACY_KEYCHAIN_SERVICE, &user)
            .and_then(|blob| extract_access_token(&blob))
    {
        return Some(t);
    }
    plaintext_token(acc_dir)
}

/// Whether a missed scoped Keychain lookup for `acc_dir` should retry
/// against the bare legacy service. Only the standard account — see
/// `read_token` for why a managed account must not fall back this way.
fn should_try_legacy_keychain_fallback(acc_dir: &Path) -> bool {
    standard_token_dir().as_deref() == Some(acc_dir)
}

/// macOS Keychain service name Claude Code stores the OAuth token under for a
/// given config dir: "Claude Code-credentials-<sha256(path)[0:8]>".
fn keychain_service(acc_dir: &Path) -> Option<String> {
    let acc_str = acc_dir.to_str()?;
    let mut hasher = Sha256::new();
    hasher.update(acc_str.as_bytes());
    let digest = hasher.finalize();
    let hash: String = digest
        .iter()
        .take(4)
        .map(|b| format!("{:02x}", b))
        .collect();
    Some(format!("Claude Code-credentials-{}", hash))
}

/// Raw credential blob (`{"claudeAiOauth":{...}}`) from the Keychain for a dir,
/// or `None` if absent / not macOS.
fn keychain_blob(acc_dir: &Path) -> Option<String> {
    let service = keychain_service(acc_dir)?;
    let user = whoami_short()?;
    read_keychain_blob(&service, &user)
}

fn keychain_token(acc_dir: &Path) -> Option<String> {
    extract_access_token(&keychain_blob(acc_dir)?)
}

fn read_keychain_blob(service: &str, user: &str) -> Option<String> {
    if !cfg!(target_os = "macos") {
        return None;
    }
    let out = Command::new("security")
        .args(["find-generic-password", "-s", service, "-a", user, "-w"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let raw = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if raw.is_empty() { None } else { Some(raw) }
}

fn write_keychain_blob(service: &str, user: &str, blob: &str) -> std::io::Result<()> {
    // `-U` updates the entry if one already exists for this service+account.
    let status = Command::new("security")
        .args(["add-generic-password", "-U"])
        .args(["-s", service, "-a", user, "-w", blob])
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(std::io::Error::other(
            "security add-generic-password failed",
        ))
    }
}

fn delete_keychain_blob(service: &str, user: &str) {
    // Best-effort: nothing to delete is not an error.
    let _ = Command::new("security")
        .args(["delete-generic-password", "-s", service, "-a", user])
        .output();
}

/// Re-key the Keychain OAuth entry from `from_dir`'s service name to
/// `to_dir`'s. Claude Code keys the token by the absolute config-dir path, so
/// relocating a config dir orphans its token from the new path — `import` calls
/// this to move it. Returns `Ok(true)` if an entry was copied, `Ok(false)` if
/// there was nothing to copy (no entry, or not macOS — the plaintext
/// `.credentials.json` fallback covers those). `Err` only on a write failure.
pub fn copy_keychain_entry(from_dir: &Path, to_dir: &Path) -> std::io::Result<bool> {
    if !cfg!(target_os = "macos") {
        return Ok(false);
    }
    let Some(blob) = keychain_blob(from_dir) else {
        return Ok(false);
    };
    let (Some(to_service), Some(user)) = (keychain_service(to_dir), whoami_short()) else {
        return Ok(false);
    };
    write_keychain_blob(&to_service, &user, &blob)?;
    Ok(true)
}

/// Keychain service names a `claude auth login` run can write to as a side
/// effect regardless of the CLAUDE_CONFIG_DIR it was scoped to: the bare
/// pre-2.1 unscoped service, and the standard `~/.claude` account's own
/// scoped entry. Observed in practice: both can exist simultaneously holding
/// *different* tokens, meaning some Claude Code login/refresh path writes to
/// one without the other staying in sync. `snapshot_side_effect_keychain` /
/// `restore_side_effect_keychain` bracket `add`/`login` for a *different*
/// account so that collateral write can't clobber the standard account.
fn side_effect_keychain_services() -> Vec<String> {
    let mut services = vec![LEGACY_KEYCHAIN_SERVICE.to_string()];
    if let Some(dir) = standard_token_dir()
        && let Some(service) = keychain_service(&dir)
        && !services.contains(&service)
    {
        services.push(service);
    }
    services
}

/// A point-in-time capture of the standard account's Keychain entries, taken
/// before logging in to a different account. `None` per-service means the
/// entry didn't exist and should be deleted (not just left alone) on restore.
pub struct KeychainSnapshot(Vec<(String, Option<String>)>);

pub fn snapshot_side_effect_keychain() -> KeychainSnapshot {
    if !cfg!(target_os = "macos") {
        return KeychainSnapshot(Vec::new());
    }
    let Some(user) = whoami_short() else {
        return KeychainSnapshot(Vec::new());
    };
    let entries = side_effect_keychain_services()
        .into_iter()
        .map(|service| {
            let blob = read_keychain_blob(&service, &user);
            (service, blob)
        })
        .collect();
    KeychainSnapshot(entries)
}

pub fn restore_side_effect_keychain(snapshot: KeychainSnapshot) {
    let Some(user) = whoami_short() else {
        return;
    };
    for (service, blob) in snapshot.0 {
        match blob {
            Some(b) => {
                let _ = write_keychain_blob(&service, &user, &b);
            }
            None => delete_keychain_blob(&service, &user),
        }
    }
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
    pub plan: Option<String>,
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
        plan: v.get("plan").and_then(|x| x.as_str()).map(String::from),
    })
}

pub fn write_cache_at(path: &Path, profile: &Profile, token: &str) -> std::io::Result<()> {
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
        "plan": profile.plan,
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

pub fn fetch_profile(token: &str) -> Option<Profile> {
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
        plan: derive_plan(&v),
    })
}

/// Build a friendly plan label from a `/oauth/profile` response: "Max 20x",
/// "Max", "Pro", or `None`. The base tier comes from the boolean flags; the
/// multiplier (e.g. "20x") is pulled out of `organization.rate_limit_tier`
/// (e.g. "default_claude_max_20x") when present.
fn derive_plan(v: &serde_json::Value) -> Option<String> {
    let account = v.get("account");
    let has_max = account
        .and_then(|a| a.get("has_claude_max"))
        .and_then(|b| b.as_bool())
        .unwrap_or(false);
    let has_pro = account
        .and_then(|a| a.get("has_claude_pro"))
        .and_then(|b| b.as_bool())
        .unwrap_or(false);
    let tier = v
        .get("organization")
        .and_then(|o| o.get("rate_limit_tier"))
        .and_then(|t| t.as_str())
        .unwrap_or("");

    if has_max {
        Some(match tier_multiplier(tier) {
            Some(m) => format!("Max {}", m),
            None => "Max".to_string(),
        })
    } else if has_pro {
        Some("Pro".to_string())
    } else {
        None
    }
}

/// Extract a `<digits>x` multiplier token (e.g. "20x") from an underscore-joined
/// rate-limit tier string. Returns `None` if no such token is present.
fn tier_multiplier(tier: &str) -> Option<String> {
    tier.split('_')
        .find(|tok| {
            let b = tok.as_bytes();
            b.len() >= 2
                && b[b.len() - 1] == b'x'
                && b[..b.len() - 1].iter().all(u8::is_ascii_digit)
        })
        .map(|s| s.to_string())
}

// --- Usage (5-hour / 7-day rate-limit windows) ---
//
// Queries the undocumented /api/oauth/usage endpoint, which returns the
// utilization (0–100 percent) and reset timestamp for each rate-limit window.
// We surface the two windows users care about: `five_hour` and `seven_day`.
// Like the profile endpoint, this is reverse-engineered and may change.

pub struct UsageWindow {
    /// Percentage of the window consumed, 0–100. The API returns a float.
    pub utilization: f64,
    /// ISO-8601 reset timestamp (e.g. "2026-06-10T12:20:01.254509+00:00"),
    /// or `None` if the window has no scheduled reset.
    pub resets_at: Option<String>,
}

pub struct Usage {
    pub five_hour: Option<UsageWindow>,
    pub seven_day: Option<UsageWindow>,
}

pub enum UsageResult {
    Ok(Usage),
    NoToken,
    Offline,
}

/// Read the token for `token_dir` (a managed account dir or `~/.claude/`) and
/// fetch its live usage. Mirrors `audit_at` but for the usage endpoint — and
/// deliberately writes no cache, since usage is volatile and only meaningful
/// fresh.
pub fn fetch_account_usage(token_dir: &Path) -> UsageResult {
    let Some(token) = read_token(token_dir) else {
        return UsageResult::NoToken;
    };
    match fetch_usage(&token) {
        Some(u) => UsageResult::Ok(u),
        None => UsageResult::Offline,
    }
}

pub fn fetch_usage(token: &str) -> Option<Usage> {
    let out = Command::new("curl")
        .args(["-sf", "--max-time", "5"])
        .args(["-H", &format!("Authorization: Bearer {}", token)])
        .args(["-H", "anthropic-beta: oauth-2025-04-20"])
        .arg("https://api.anthropic.com/api/oauth/usage")
        .output()
        .ok()?;
    if !out.status.success() || out.stdout.is_empty() {
        return None;
    }
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).ok()?;
    Some(Usage {
        five_hour: parse_window(v.get("five_hour")),
        seven_day: parse_window(v.get("seven_day")),
    })
}

fn parse_window(v: Option<&serde_json::Value>) -> Option<UsageWindow> {
    let v = v?;
    if v.is_null() {
        return None;
    }
    Some(UsageWindow {
        utilization: v.get("utilization").and_then(|u| u.as_f64()).unwrap_or(0.0),
        resets_at: v
            .get("resets_at")
            .and_then(|r| r.as_str())
            .map(String::from),
    })
}

/// Seconds from now until `resets_at`. Returns `None` if the timestamp can't be
/// parsed; a value `<= 0` means the window has already reset.
pub fn seconds_until(resets_at: &str) -> Option<i64> {
    let target = iso_to_epoch(resets_at)?;
    let now = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs() as i64;
    Some(target - now)
}

/// Parse an ISO-8601 timestamp like `2026-06-10T12:20:01.254509+00:00` (or
/// trailing `Z`) into Unix epoch seconds. Fractional seconds are ignored.
/// Returns `None` on any malformed component.
fn iso_to_epoch(s: &str) -> Option<i64> {
    let (date, rest) = s.split_once('T')?;
    let mut dparts = date.split('-');
    let y: i64 = dparse(dparts.next())?;
    let m: i64 = dparse(dparts.next())?;
    let d: i64 = dparse(dparts.next())?;

    // Split the time from its timezone suffix.
    let (time, offset_secs) = if let Some(t) = rest.strip_suffix('Z') {
        (t, 0i64)
    } else if let Some(pos) = rest.rfind(['+', '-']) {
        let (t, off) = rest.split_at(pos);
        (t, parse_offset(off)?)
    } else {
        (rest, 0i64)
    };

    let mut tparts = time.split(':');
    let hh: i64 = dparse(tparts.next())?;
    let mm: i64 = dparse(tparts.next())?;
    // Seconds may carry a fractional part — keep only the integer seconds.
    let sec_field = tparts.next()?;
    let ss: i64 = dparse(Some(sec_field.split('.').next().unwrap_or(sec_field)))?;

    Some(days_from_civil(y, m, d) * 86_400 + hh * 3_600 + mm * 60 + ss - offset_secs)
}

/// Parse a `+HH:MM` / `-HH:MM` timezone offset into signed seconds.
fn parse_offset(off: &str) -> Option<i64> {
    let sign = match off.as_bytes().first()? {
        b'+' => 1,
        b'-' => -1,
        _ => return None,
    };
    let body = &off[1..];
    let (oh, om) = body.split_once(':')?;
    let oh: i64 = dparse(Some(oh))?;
    let om: i64 = dparse(Some(om))?;
    Some(sign * (oh * 3_600 + om * 60))
}

fn dparse(s: Option<&str>) -> Option<i64> {
    s?.parse().ok()
}

/// Days since the Unix epoch (1970-01-01) for a proleptic-Gregorian date.
/// Howard Hinnant's `days_from_civil` algorithm.
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_hash_is_deterministic() {
        let a = token_hash("the-quick-brown-fox");
        let b = token_hash("the-quick-brown-fox");
        assert_eq!(a, b);
    }

    #[test]
    fn token_hash_diverges_on_different_input() {
        assert_ne!(token_hash("foo"), token_hash("bar"));
    }

    #[test]
    fn token_hash_is_16_hex_chars() {
        let h = token_hash("anything");
        assert_eq!(h.len(), 16, "got {h:?}");
        assert!(
            h.chars().all(|c| c.is_ascii_hexdigit()),
            "non-hex char in {h:?}"
        );
    }

    #[test]
    fn side_effect_keychain_services_include_bare_legacy_name() {
        let services = side_effect_keychain_services();
        assert!(services.contains(&"Claude Code-credentials".to_string()));
    }

    #[test]
    fn legacy_keychain_fallback_applies_only_to_the_standard_account() {
        let standard_dir = standard_token_dir().expect("home dir should resolve in CI");
        assert!(should_try_legacy_keychain_fallback(&standard_dir));

        assert!(!should_try_legacy_keychain_fallback(Path::new(
            "/tmp/some-managed-account"
        )));
        assert!(!should_try_legacy_keychain_fallback(
            standard_dir.parent().expect("home dir has a parent")
        ));
    }

    #[test]
    fn side_effect_keychain_services_has_no_duplicates() {
        let services = side_effect_keychain_services();
        let mut deduped = services.clone();
        deduped.sort();
        deduped.dedup();
        assert_eq!(services.len(), deduped.len(), "got {services:?}");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn keychain_blob_roundtrip_write_read_delete() {
        // A service name distinctive enough that it can never collide with a
        // real Claude Code Keychain entry.
        let service = "claude-acc-test-keychain-roundtrip";
        let user = whoami_short().expect("whoami should succeed in CI");
        delete_keychain_blob(service, &user); // clean slate, in case a prior run left one

        assert_eq!(read_keychain_blob(service, &user), None);

        write_keychain_blob(service, &user, "hello-world").expect("write should succeed");
        assert_eq!(
            read_keychain_blob(service, &user),
            Some("hello-world".to_string())
        );

        // `-U` semantics: writing again updates rather than erroring.
        write_keychain_blob(service, &user, "updated").expect("update should succeed");
        assert_eq!(
            read_keychain_blob(service, &user),
            Some("updated".to_string())
        );

        delete_keychain_blob(service, &user);
        assert_eq!(read_keychain_blob(service, &user), None);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn keychain_snapshot_restore_roundtrips_absent_and_present_entries() {
        // Exercises the snapshot/restore *shape* (present -> restore value,
        // absent -> restore to absent) against fake services, without ever
        // touching the real "Claude Code-credentials" entries this function
        // is meant to protect in production.
        let user = whoami_short().expect("whoami should succeed in CI");
        let present_service = "claude-acc-test-keychain-snapshot-present";
        let absent_service = "claude-acc-test-keychain-snapshot-absent";
        delete_keychain_blob(present_service, &user);
        delete_keychain_blob(absent_service, &user);
        write_keychain_blob(present_service, &user, "original").expect("seed write");

        let snapshot = KeychainSnapshot(vec![
            (
                present_service.to_string(),
                read_keychain_blob(present_service, &user),
            ),
            (
                absent_service.to_string(),
                read_keychain_blob(absent_service, &user),
            ),
        ]);

        // Simulate the collateral damage `claude auth login` can cause.
        write_keychain_blob(present_service, &user, "clobbered").expect("clobber write");
        write_keychain_blob(absent_service, &user, "clobbered").expect("clobber write");

        restore_side_effect_keychain(snapshot);

        assert_eq!(
            read_keychain_blob(present_service, &user),
            Some("original".to_string())
        );
        assert_eq!(read_keychain_blob(absent_service, &user), None);

        delete_keychain_blob(present_service, &user);
        delete_keychain_blob(absent_service, &user);
    }

    #[test]
    fn seconds_since_rejects_zero() {
        assert_eq!(seconds_since(0), None);
    }

    #[test]
    fn seconds_since_rejects_future_timestamp() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert_eq!(seconds_since(now + 86_400), None);
    }

    #[test]
    fn seconds_since_returns_diff_for_past_timestamp() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        // 100 seconds ago — give the diff a 5-second slack for slow runners.
        let diff = seconds_since(now - 100).expect("expected Some");
        assert!((95..=105).contains(&diff), "got diff {diff}");
    }

    #[test]
    fn read_cache_at_missing_returns_none() {
        let p = std::path::PathBuf::from("/definitely/does/not/exist/.account-info.json");
        assert!(read_cache_at(&p).is_none());
    }

    #[test]
    fn iso_to_epoch_utc_offset() {
        // 2026-06-10T12:20:01+00:00 == 1781094001
        // (verified via `TZ=UTC date -j -f %Y-%m-%dT%H:%M:%S ... +%s`).
        assert_eq!(
            iso_to_epoch("2026-06-10T12:20:01.254509+00:00"),
            Some(1_781_094_001)
        );
    }

    #[test]
    fn iso_to_epoch_z_suffix_matches_offset() {
        assert_eq!(
            iso_to_epoch("2026-06-10T12:20:01Z"),
            iso_to_epoch("2026-06-10T12:20:01+00:00")
        );
    }

    #[test]
    fn iso_to_epoch_applies_nonzero_offset() {
        // +02:00 is two hours ahead, so the same wall clock is 7200s earlier in UTC.
        let utc = iso_to_epoch("2026-06-10T12:20:01+00:00").unwrap();
        let plus2 = iso_to_epoch("2026-06-10T12:20:01+02:00").unwrap();
        assert_eq!(utc - plus2, 7_200);
    }

    #[test]
    fn iso_to_epoch_epoch_zero() {
        assert_eq!(iso_to_epoch("1970-01-01T00:00:00Z"), Some(0));
    }

    #[test]
    fn iso_to_epoch_rejects_garbage() {
        assert_eq!(iso_to_epoch("not-a-timestamp"), None);
        assert_eq!(iso_to_epoch(""), None);
    }

    #[test]
    fn tier_multiplier_extracts_token() {
        assert_eq!(
            tier_multiplier("default_claude_max_20x"),
            Some("20x".to_string())
        );
        assert_eq!(
            tier_multiplier("default_claude_max_5x"),
            Some("5x".to_string())
        );
    }

    #[test]
    fn tier_multiplier_none_when_absent() {
        assert_eq!(tier_multiplier("default_claude_pro"), None);
        assert_eq!(tier_multiplier(""), None);
        // "x" alone or non-digit prefixes must not match.
        assert_eq!(tier_multiplier("x"), None);
        assert_eq!(tier_multiplier("maxx"), None);
    }

    #[test]
    fn derive_plan_max_with_multiplier() {
        let v = serde_json::json!({
            "account": {"has_claude_max": true, "has_claude_pro": false},
            "organization": {"rate_limit_tier": "default_claude_max_20x"}
        });
        assert_eq!(derive_plan(&v), Some("Max 20x".to_string()));
    }

    #[test]
    fn derive_plan_max_without_multiplier() {
        let v = serde_json::json!({
            "account": {"has_claude_max": true},
            "organization": {"rate_limit_tier": "default_claude_max"}
        });
        assert_eq!(derive_plan(&v), Some("Max".to_string()));
    }

    #[test]
    fn derive_plan_pro() {
        let v = serde_json::json!({
            "account": {"has_claude_max": false, "has_claude_pro": true},
            "organization": {"rate_limit_tier": "default_claude_pro"}
        });
        assert_eq!(derive_plan(&v), Some("Pro".to_string()));
    }

    #[test]
    fn derive_plan_none_when_neither() {
        let v = serde_json::json!({
            "account": {"has_claude_max": false, "has_claude_pro": false},
            "organization": {}
        });
        assert_eq!(derive_plan(&v), None);
    }

    fn cached(email: Option<&str>, uuid: Option<&str>) -> CachedInfo {
        CachedInfo {
            email: email.map(String::from),
            uuid: uuid.map(String::from),
            org: None,
            fetched_at: None,
            token_hash: None,
            plan: None,
        }
    }

    #[test]
    fn identity_matches_by_uuid_regardless_of_email() {
        let c = cached(Some("old@example.com"), Some("uuid-1"));
        assert!(identity_matches(
            Some("uuid-1"),
            Some("new@example.com"),
            &c
        ));
    }

    #[test]
    fn identity_matches_by_email_case_insensitively_when_uuid_absent() {
        let c = cached(Some("Person@Example.com"), None);
        assert!(identity_matches(None, Some("person@example.com"), &c));
    }

    #[test]
    fn identity_matches_prefers_uuid_over_a_conflicting_email() {
        // A stale cached email shouldn't cause a false mismatch when the
        // uuid — the stable signal — actually agrees.
        let c = cached(Some("stale@example.com"), Some("uuid-1"));
        assert!(identity_matches(
            Some("uuid-1"),
            Some("current@example.com"),
            &c
        ));
    }

    #[test]
    fn identity_matches_false_when_neither_uuid_nor_email_agree() {
        let c = cached(Some("a@example.com"), Some("uuid-a"));
        assert!(!identity_matches(Some("uuid-b"), Some("b@example.com"), &c));
    }

    #[test]
    fn identity_matches_false_when_nothing_to_compare() {
        let c = cached(None, None);
        assert!(!identity_matches(None, None, &c));
    }

    fn id(uuid: &str, email: Option<&str>) -> Identity {
        Identity {
            uuid: uuid.to_string(),
            email: email.map(String::from),
        }
    }

    #[test]
    fn a_pin_that_matches_the_signed_in_account_is_ok() {
        let same = id("u-1", Some("a@example.com"));
        assert_eq!(compare_lock(Some(&same), Some(&same)), LockState::Ok);
    }

    #[test]
    fn a_different_uuid_is_drift_and_carries_both_identities() {
        // The report has to name both, or the reader cannot tell which way
        // round the swap went — "you are on the wrong account" is useless
        // without "and the right one is this".
        let pinned = id("u-1", Some("work@example.com"));
        let now = id("u-2", Some("personal@example.com"));
        match compare_lock(Some(&pinned), Some(&now)) {
            LockState::Drift { expected, actual } => {
                assert_eq!(expected, pinned);
                assert_eq!(actual, now);
            }
            other => panic!("expected drift, got {other:?}"),
        }
    }

    #[test]
    fn only_the_uuid_decides_whether_the_pin_holds() {
        // An email can change on the same account, and two accounts can share
        // a display name. Comparing on anything softer than the uuid produces
        // drift reports that are wrong in both directions.
        let pinned = id("u-1", Some("old.address@example.com"));
        let renamed = id("u-1", Some("new.address@example.com"));
        assert_eq!(compare_lock(Some(&pinned), Some(&renamed)), LockState::Ok);

        let twins_a = id("u-1", Some("same@example.com"));
        let twins_b = id("u-2", Some("same@example.com"));
        assert!(matches!(
            compare_lock(Some(&twins_a), Some(&twins_b)),
            LockState::Drift { .. }
        ));
    }

    #[test]
    fn an_unpinned_account_is_not_reported_as_drift() {
        // Every account predating this feature is unpinned. Treating that as
        // a problem would bury the real one under noise.
        assert_eq!(
            compare_lock(None, Some(&id("u-1", None))),
            LockState::NoLock
        );
        assert_eq!(compare_lock(None, None), LockState::NoLock);
    }

    #[test]
    fn a_pin_with_nobody_signed_in_is_unknown_rather_than_drift() {
        // A directory that has never been logged in has no identity to
        // compare against. That is not the same as being on the wrong one.
        assert_eq!(
            compare_lock(Some(&id("u-1", None)), None),
            LockState::Unknown
        );
    }

    #[test]
    fn the_pin_round_trips_through_disk() {
        let dir = std::env::temp_dir().join(format!("cc-lock-rt-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join(LOCK_FILE);

        let original = id("u-1", Some("a@example.com"));
        write_lock_at(&path, &original).unwrap();
        assert_eq!(read_lock_at(&path), Some(original));

        // A pin without an email still reads back — older pins and accounts
        // whose profile never carried one.
        let no_email = id("u-2", None);
        write_lock_at(&path, &no_email).unwrap();
        assert_eq!(read_lock_at(&path), Some(no_email));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_absent_or_unreadable_pin_reads_as_no_pin() {
        let dir = std::env::temp_dir().join(format!("cc-lock-bad-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        assert_eq!(read_lock_at(&dir.join(LOCK_FILE)), None);
        // Garbage, and valid JSON that simply has no uuid: both mean "no
        // usable pin", never a drift report against nothing.
        for body in ["{ broken", "{}", r#"{"email": "a@example.com"}"#] {
            fs::write(dir.join(LOCK_FILE), body).unwrap();
            assert_eq!(read_lock_at(&dir.join(LOCK_FILE)), None, "{body}");
        }

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_identity_comes_from_claude_codes_own_file_without_keychain_or_network() {
        // This is what makes the check cheap enough to run anywhere: Claude
        // Code records the signed-in account in a plain JSON file next to the
        // config dir it was given.
        let dir = std::env::temp_dir().join(format!("cc-lock-local-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        fs::write(
            dir.join(".claude.json"),
            serde_json::json!({
                "oauthAccount": { "accountUuid": "u-9", "emailAddress": "a@example.com" },
                "somethingElse": 1
            })
            .to_string(),
        )
        .unwrap();
        assert_eq!(local_identity(&dir), Some(id("u-9", Some("a@example.com"))));

        // No oauthAccount, or no uuid in it: nobody is signed in.
        fs::write(dir.join(".claude.json"), r#"{"other": 1}"#).unwrap();
        assert_eq!(local_identity(&dir), None);
        fs::write(dir.join(".claude.json"), r#"{"oauthAccount": {}}"#).unwrap();
        assert_eq!(local_identity(&dir), None);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_standard_account_keeps_its_record_beside_the_config_dir_not_inside_it() {
        // Claude Code writes `.claude.json` at `CLAUDE_CONFIG_DIR ?? $HOME`,
        // so for the un-managed account it lands next to `~/.claude/`, not in
        // it. Looking inside would find nothing and report every standard
        // account as never signed in.
        let standard = standard_token_dir().unwrap();
        let path = local_identity_path(&standard).unwrap();
        assert_eq!(path, dirs::home_dir().unwrap().join(".claude.json"));

        let managed = std::path::Path::new("/tmp/accounts/work");
        assert_eq!(
            local_identity_path(managed).unwrap(),
            managed.join(".claude.json")
        );
    }

    #[test]
    fn find_duplicate_account_none_when_new_dir_has_no_token() {
        let new_dir =
            std::env::temp_dir().join(format!("claude-acc-test-no-token-{}", std::process::id()));
        let _ = fs::remove_dir_all(&new_dir);
        fs::create_dir_all(&new_dir).unwrap();

        let result = find_duplicate_account(&new_dir, &[]);

        let _ = fs::remove_dir_all(&new_dir);
        assert_eq!(result, None);
    }
}
