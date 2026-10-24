//! A typed, secret-free view of accounts for front ends other than the CLI.
//!
//! Reads return serde types, and writes go through the same `AppConfig`
//! setters and `resolve` rules the CLI uses. This module is the contract a
//! GUI consumes. Its serialized shape is pinned by `gui.contract.json`
//! beside this file, which a test compares with what these types produce,
//! so renaming a field means updating that file in the same change.
//!
//! # What is in it
//!
//! - [`snapshot`]: every account (the standard `~/.claude` one first, as
//!   `"default"`) with identity, plan, organization, pin state and the
//!   5-hour and 7-day usage windows. `live = false` reads local records only;
//!   `live = true` asks the profile and usage APIs, one thread per account.
//! - [`bindings`]: stored directory bindings, plus the effective account for
//!   one directory and the binding it inherits from, via `resolve_account`.
//! - [`sessions`]: session metadata, newest first.
//! - [`change_default`] and [`change_binding`]: validated writes.
//! - [`remove_account`]: a managed account to the desktop trash, never
//!   deleted outright.
//! - [`pin_account`]: pin an account to the identity it is signed in as,
//!   by the same rules as `claude-acc lock`.
//! - [`check_launch`]: whether `run`, `login` or `add` may start for an
//!   account, and in which physical directory. The decision only; launching
//!   is the caller's.
//!
//! # Time
//!
//! Usage carries `fetched_at` and each window's `resets_at` as Unix seconds,
//! not an age. A consumer ages the reading against its own clock and stops
//! showing a window's figure once `resets_at` has passed, so nothing on
//! screen is frozen at the moment the snapshot was built.
//!
//! # Errors
//!
//! Every failure is a [`GuiError`], which serializes as a `snake_case` code
//! for the consumer to word. A code never carries a path, subprocess output
//! or credential data.
//!
//! # Deliberately absent
//!
//! No token, token hash, credential file, Keychain entry, raw config file or
//! transcript path or content is serialized, by construction: the types
//! have no field that could hold one. A symlink planted in the storage (an
//! account directory, a credential or identity file, `config`, `links`) is
//! refused before anything is read or written; the storage roots themselves,
//! and anything above them, may be symlinks. There is no notion of a global
//! "active" account; `is_default` is only the configured fallback for
//! unbound directories.
use crate::config::{AppConfig, validate_name};
use crate::{identity, resolve, sessions, trash};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Machine codes a consumer translates into copy. Never carries a path,
/// subprocess output or credential data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GuiError {
    InvalidAccount,
    AccountMissing,
    AccountExists,
    SymlinkStorage,
    StorageUnreadable,
    StorageWriteFailed,
    InvalidDirectory,
    UnrepresentableDirectory,
    DirectoryMissing,
    SharedCredentialStore,
    IdentityDrift,
    TrashUnavailable,
    NotSignedIn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IdentitySource {
    /// Claude Code's own record on disk; no network.
    Local,
    /// Confirmed by the profile API during this refresh.
    Live,
    Unknown,
    Offline,
    NoCredentials,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LockStatus {
    Matched,
    Drift,
    Unpinned,
    Unknown,
    Corrupt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UsageSource {
    Live,
    /// Claude Code's own last reading, used only when the API failed or was
    /// not asked.
    Cached,
    NoCredentials,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct UsageWindow {
    pub percent: f64,
    /// Unix seconds. The reading means nothing once this has passed; the
    /// consumer compares it against its own clock, not against snapshot time.
    pub resets_at: Option<u64>,
}

impl UsageWindow {
    pub fn reset_passed(&self, now: u64) -> bool {
        self.resets_at.is_some_and(|r| r <= now)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Usage {
    pub source: UsageSource,
    /// Unix seconds when the reading was taken — not when the snapshot was
    /// built — so its age can keep growing on screen. `None` without one.
    pub fetched_at: Option<u64>,
    pub five_hour: Option<UsageWindow>,
    pub seven_day: Option<UsageWindow>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Account {
    pub name: String,
    pub email: Option<String>,
    pub plan: Option<String>,
    pub organization: Option<String>,
    pub is_default: bool,
    pub available: bool,
    pub identity_source: IdentitySource,
    pub lock: LockStatus,
    pub usage: Usage,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Snapshot {
    /// Unix seconds when the snapshot was built.
    pub observed_at: u64,
    pub default_account: String,
    /// The standard account first, then managed accounts by name.
    pub accounts: Vec<Account>,
}

impl Snapshot {
    /// Reflect a default change without re-reading every account — and
    /// without discarding the live readings an earlier refresh paid for.
    pub fn mark_default(&mut self, name: &str) {
        self.default_account = name.to_string();
        for a in &mut self.accounts {
            a.is_default = a.name == name;
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Binding {
    pub directory: String,
    pub account: String,
    pub exists: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Selection {
    /// The physical directory, as the CLI's `current_dir()` would see it.
    pub directory: String,
    pub account: String,
    pub inherited_from: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Bindings {
    pub bindings: Vec<Binding>,
    pub selected: Option<Selection>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Session {
    pub id: String,
    pub account: String,
    pub project: String,
    pub modified: Option<u64>,
    pub size: u64,
    pub duplicate: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LaunchAction {
    Run,
    Login,
    Add,
}

impl LaunchAction {
    /// The `claude-acc` subcommand this action runs.
    pub fn command(self) -> &'static str {
        match self {
            Self::Run => "run",
            Self::Login => "login",
            Self::Add => "add",
        }
    }
}

pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn check_readable(path: &Path) -> Result<(), GuiError> {
    match fs::read_to_string(path) {
        Ok(_) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err(GuiError::StorageUnreadable),
    }
}

/// Whether a refresh may run, given `CLAUDE_SECURESTORAGE_CONFIG_DIR` as
/// Claude Code sees it (`None` only when unset). Set at all, every account
/// reads the same Keychain entry, or with an empty value the bare legacy one,
/// so a live refresh would report one identity's figures for every account;
/// `doctor`'s `shared_credential_store` reads the variable the same way. A
/// local read is unaffected.
fn live_allowed(live: bool, securestorage: Option<&str>) -> Result<(), GuiError> {
    match (live, securestorage) {
        (true, Some(_)) => Err(GuiError::SharedCredentialStore),
        _ => Ok(()),
    }
}

fn account_path(config: &AppConfig, standard: &Path, name: &str) -> PathBuf {
    if name == "default" {
        standard.to_path_buf()
    } else {
        config.account_path(name)
    }
}

fn lock_path(config: &AppConfig, name: &str, dir: &Path) -> PathBuf {
    if name == "default" {
        identity::default_lock_path(&config.base_dir)
    } else {
        dir.join(identity::LOCK_FILE)
    }
}

fn cache_path(config: &AppConfig, name: &str, dir: &Path) -> PathBuf {
    if name == "default" {
        identity::default_cache_path(&config.base_dir)
    } else {
        dir.join(".account-info.json")
    }
}

/// The directory an account's storage hangs from: the switch directory for a
/// managed account, `~/.claude` itself for the standard one.
fn storage_root<'a>(config: &'a AppConfig, standard: &'a Path, name: &str) -> &'a Path {
    if name == "default" {
        standard
    } else {
        &config.base_dir
    }
}

/// An existing account directory whose storage is safe to read secrets from.
fn account_dir(config: &AppConfig, standard: &Path, name: &str) -> Result<PathBuf, GuiError> {
    if !validate_name(name) {
        return Err(GuiError::InvalidAccount);
    }
    let path = account_path(config, standard, name);
    if !path.is_dir() {
        return Err(GuiError::AccountMissing);
    }
    let root = storage_root(config, standard, name);
    reject_symlinks(root, &path)?;
    for file in [
        ".credentials.json",
        ".account-info.json",
        identity::LOCK_FILE,
    ] {
        reject_symlinks(root, &path.join(file))?;
    }
    if let Some(local) = identity::local_identity_path(&path) {
        // The standard account's `.claude.json` sits beside `~/.claude`, in
        // the home directory, not inside it.
        let local_root = local.parent().ok_or(GuiError::StorageUnreadable)?;
        reject_symlinks(local_root, &local)?;
    }
    Ok(path)
}

/// Refuse a symlink planted in the storage: any component of `path` strictly
/// below `root`. `root` itself, and everything above it, may be symlinks: a
/// home on another volume, `/home` → `/Users`, a Nix or home-manager layout,
/// or a dotfiles `~/.claude-switch`. Those are where the storage lives, not
/// something pointing out of it, and the CLI works in all of them.
fn reject_symlinks(root: &Path, path: &Path) -> Result<(), GuiError> {
    if !path.starts_with(root) {
        // A caller bug: fail closed rather than check nothing.
        return Err(GuiError::StorageUnreadable);
    }
    for ancestor in path.ancestors().take_while(|a| *a != root) {
        match fs::symlink_metadata(ancestor) {
            Ok(meta) if meta.file_type().is_symlink() => return Err(GuiError::SymlinkStorage),
            Ok(_) => (),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
            Err(_) => return Err(GuiError::StorageUnreadable),
        }
    }
    Ok(())
}

fn project_path(raw: &str, must_exist: bool) -> Result<PathBuf, GuiError> {
    let path = PathBuf::from(raw);
    if !path.is_absolute() || raw.chars().any(char::is_control) || raw.trim() != raw {
        return Err(GuiError::InvalidDirectory);
    }
    if path
        .components()
        .any(|c| c == std::path::Component::ParentDir)
    {
        return Err(GuiError::UnrepresentableDirectory);
    }
    if !must_exist {
        return Ok(path);
    }
    // Match the physical cwd returned to the CLI by std::env::current_dir().
    path.canonicalize()
        .ok()
        .filter(|p| p.is_dir())
        .ok_or(GuiError::DirectoryMissing)
}

fn window(w: &identity::UsageWindow) -> UsageWindow {
    UsageWindow {
        percent: w.utilization,
        resets_at: w
            .resets_at
            .as_deref()
            .and_then(identity::iso_to_epoch)
            .and_then(|epoch| u64::try_from(epoch).ok()),
    }
}

fn usage(result: identity::UsageResult, now: u64) -> Usage {
    let (source, reading, fetched_at) = match result {
        identity::UsageResult::Ok(u) => (UsageSource::Live, Some(u), Some(now)),
        identity::UsageResult::Cached(u, age) => {
            (UsageSource::Cached, Some(u), Some(now.saturating_sub(age)))
        }
        identity::UsageResult::NoToken => (UsageSource::NoCredentials, None, None),
        identity::UsageResult::Offline => (UsageSource::Unavailable, None, None),
    };
    Usage {
        source,
        fetched_at,
        five_hour: reading
            .as_ref()
            .and_then(|u| u.five_hour.as_ref())
            .map(window),
        seven_day: reading
            .as_ref()
            .and_then(|u| u.seven_day.as_ref())
            .map(window),
    }
}

/// The pin state of one account, read from disk now. Cheap: two local
/// files, no network. For a consumer that keeps a snapshot and must not
/// show a pin state older than a change it just made.
pub fn lock_of(config: &AppConfig, standard: &Path, name: &str) -> LockStatus {
    // Storage that would be refused for a snapshot is not read here either.
    let Ok(dir) = account_dir(config, standard, name) else {
        return LockStatus::Unknown;
    };
    let lock = lock_path(config, name, &dir);
    if reject_symlinks(&config.base_dir, &lock).is_err() {
        return LockStatus::Unknown;
    }
    lock_status(identity::lock_state_at(&lock, &dir))
}

fn lock_status(state: identity::LockState) -> LockStatus {
    match state {
        identity::LockState::Ok => LockStatus::Matched,
        identity::LockState::Drift { .. } => LockStatus::Drift,
        identity::LockState::NoLock => LockStatus::Unpinned,
        identity::LockState::Unknown => LockStatus::Unknown,
        identity::LockState::Corrupt => LockStatus::Corrupt,
    }
}

/// One account row. With `live`, this makes the profile and usage requests —
/// several seconds each when the network is slow.
fn account(
    config: &AppConfig,
    standard: &Path,
    name: &str,
    default: &str,
    live: bool,
    now: u64,
) -> Account {
    let path = account_path(config, standard, name);
    let mut row = Account {
        name: name.to_string(),
        email: None,
        plan: None,
        organization: None,
        is_default: name == default,
        available: false,
        identity_source: IdentitySource::Unknown,
        lock: LockStatus::Unknown,
        usage: usage(identity::UsageResult::Offline, now),
    };
    if account_dir(config, standard, name).is_err() {
        return row;
    }
    row.available = true;
    let cache_path = cache_path(config, name, &path);
    let local = identity::local_identity(&path);
    // The profile cache is shown only while it describes whoever is signed
    // in now; after a re-login it describes someone else.
    let cache = reject_symlinks(&config.base_dir, &cache_path)
        .ok()
        .and_then(|_| identity::read_cache_at(&cache_path))
        .filter(|c| {
            c.uuid.is_some() && c.uuid.as_deref() == local.as_ref().map(|l| l.uuid.as_str())
        });
    row.email = local.as_ref().and_then(|l| l.email.clone());
    row.plan = cache.as_ref().and_then(|c| c.plan.clone());
    row.organization = cache.and_then(|c| c.org);
    if local.is_some() {
        row.identity_source = IdentitySource::Local;
    }
    row.lock = lock_of(config, standard, name);
    if !live {
        row.usage = usage(
            identity::cached_usage(&path).map_or(identity::UsageResult::Offline, |(u, age)| {
                identity::UsageResult::Cached(u, age)
            }),
            now,
        );
        return row;
    }
    match identity::audit_at(&path, &cache_path) {
        identity::AuditResult::Ok(p) => {
            row.email = p.email;
            row.plan = p.plan;
            row.organization = p.organization;
            row.identity_source = IdentitySource::Live;
        }
        identity::AuditResult::NoToken => row.identity_source = IdentitySource::NoCredentials,
        identity::AuditResult::Offline => row.identity_source = IdentitySource::Offline,
    }
    row.usage = usage(identity::fetch_account_usage(&path), now);
    row
}

/// Every account, the standard one first. `live` asks the profile and usage
/// APIs, one thread per account so a slow network costs one timeout rather
/// than one per account; otherwise only local records are read.
pub fn snapshot(config: &AppConfig, standard: &Path, live: bool) -> Result<Snapshot, GuiError> {
    for path in [
        config.config_path(),
        config.links_path(),
        config.accounts_dir(),
        identity::default_cache_path(&config.base_dir),
        identity::default_lock_path(&config.base_dir),
    ] {
        reject_symlinks(&config.base_dir, &path)?;
    }
    // Read here, decided in `live_allowed`, so the decision is testable
    // without touching the environment of a shared test process.
    live_allowed(live, identity::securestorage_override().as_deref())?;
    check_readable(&config.config_path())?;
    let default = config
        .get_default()
        .map_err(|_| GuiError::StorageUnreadable)?
        .unwrap_or_else(|| "default".into());
    let mut names = config
        .list_accounts()
        .map_err(|_| GuiError::StorageUnreadable)?;
    names.retain(|n| validate_name(n) && n != "default");
    names.insert(0, "default".into());
    let now = now();
    let row = |name: &String| account(config, standard, name, &default, live, now);
    // TODO: one thread per account, unbounded; cap the fan-out if setups with
    // dozens of accounts appear.
    let accounts = if live {
        std::thread::scope(|s| {
            let probes: Vec<_> = names.iter().map(|n| s.spawn(move || row(n))).collect();
            probes
                .into_iter()
                .map(|p| p.join().unwrap_or_else(|e| std::panic::resume_unwind(e)))
                .collect()
        })
    } else {
        names.iter().map(row).collect()
    };
    Ok(Snapshot {
        observed_at: now,
        default_account: default,
        accounts,
    })
}

pub fn bindings(config: &AppConfig, selected: Option<&str>) -> Result<Bindings, GuiError> {
    reject_symlinks(&config.base_dir, &config.links_path())?;
    let mut links = match config.all_links() {
        Ok(links) => links,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => vec![],
        Err(_) => return Err(GuiError::StorageUnreadable),
    };
    links.sort();
    let selected = selected.map(|p| project_path(p, true)).transpose()?;
    Ok(Bindings {
        bindings: links
            .into_iter()
            .map(|(directory, account)| Binding {
                exists: Path::new(&directory).is_dir(),
                directory,
                account,
            })
            .collect(),
        selected: selected.map(|p| Selection {
            directory: p.to_string_lossy().into_owned(),
            account: resolve::resolve_account(config, &p).unwrap_or_else(|| "default".into()),
            inherited_from: resolve::find_linked_dir(config, &p),
        }),
    })
}

/// Session metadata across every safe account, newest first. Transcript
/// paths and content stay on the Rust side.
pub fn sessions(config: &AppConfig, standard: &Path) -> Result<Vec<Session>, GuiError> {
    let names = config
        .list_accounts()
        .map_err(|_| GuiError::StorageUnreadable)?;
    let mut all = Vec::new();
    for name in std::iter::once("default".to_string()).chain(names) {
        if let Ok(dir) = account_dir(config, standard, &name)
            && reject_symlinks(
                storage_root(config, standard, &name),
                &sessions::projects_dir(&dir),
            )
            .is_ok()
        {
            all.extend(sessions::list_in(&dir, &name, None));
        }
    }
    sessions::sort_newest_first(&mut all);
    let duplicate: HashSet<String> = sessions::duplicated_ids(&all).into_iter().collect();
    Ok(all
        .into_iter()
        .map(|s| Session {
            duplicate: duplicate.contains(&s.id),
            id: s.id,
            account: s.account,
            project: s.project,
            modified: s.modified,
            size: s.size,
        })
        .collect())
}

pub fn change_default(config: &AppConfig, standard: &Path, name: &str) -> Result<(), GuiError> {
    if name != "default" {
        account_dir(config, standard, name)?;
    }
    writable(config)?;
    if name == "default" {
        config.clear_default()
    } else {
        config.set_default(name)
    }
    .map_err(|_| GuiError::StorageWriteFailed)
}

pub fn change_binding(
    config: &AppConfig,
    standard: &Path,
    directory: &str,
    account: Option<&str>,
) -> Result<(), GuiError> {
    let path = project_path(directory, account.is_some())?;
    // TODO: the line-based `links` format cannot represent '=' in a path;
    // lifting this needs escaping in both the CLI and claude-switch.sh first.
    if account.is_some() && path.to_string_lossy().contains('=') {
        return Err(GuiError::UnrepresentableDirectory);
    }
    if let Some(name) = account
        && name != "default"
    {
        account_dir(config, standard, name)?;
    }
    writable(config)?;
    let p = path.to_str().ok_or(GuiError::InvalidDirectory)?;
    match account {
        Some(name) => config.set_link(p, name),
        None => config.remove_link(p).map(|_| ()),
    }
    .map_err(|_| GuiError::StorageWriteFailed)
}

/// Move a managed account to the desktop trash, then drop the default and
/// the bindings that named it, as `claude-acc remove` does.
///
/// Unlike the CLI there is no fallback to deleting outright: a window that
/// said "Trash" must not purge. If the move fails, nothing has changed.
pub fn remove_account(config: &AppConfig, standard: &Path, name: &str) -> Result<(), GuiError> {
    remove_account_with(config, standard, name, trash::trash_dir)
}

fn remove_account_with(
    config: &AppConfig,
    standard: &Path,
    name: &str,
    to_trash: impl FnOnce(&Path) -> std::io::Result<PathBuf>,
) -> Result<(), GuiError> {
    // The standard account is `~/.claude` itself, not ours to remove.
    if name == "default" {
        return Err(GuiError::InvalidAccount);
    }
    let dir = account_dir(config, standard, name)?;
    writable(config)?;
    to_trash(&dir).map_err(|_| GuiError::TrashUnavailable)?;
    if config.get_default().ok().flatten().as_deref() == Some(name) {
        config
            .clear_default()
            .map_err(|_| GuiError::StorageWriteFailed)?;
    }
    config
        .remove_links_for_account(name)
        .map_err(|_| GuiError::StorageWriteFailed)
}

/// Pin an account to the identity it is signed in as now, as `claude-acc
/// lock` does. Pinning the identity it is already pinned to does nothing.
/// A pin to someone else, or one that cannot be read, is replaced only with
/// `force`: re-pinning is how a drift warning gets switched off.
pub fn pin_account(
    config: &AppConfig,
    standard: &Path,
    name: &str,
    force: bool,
) -> Result<(), GuiError> {
    let dir = account_dir(config, standard, name)?;
    let lock = lock_path(config, name, &dir);
    // Every pin lives under the switch directory, the standard account's
    // included, which `account_dir` does not cover.
    reject_symlinks(&config.base_dir, &lock)?;
    writable(config)?;
    let current = identity::local_identity(&dir).ok_or(GuiError::NotSignedIn)?;
    match identity::plan_lock(&identity::read_lock_at(&lock), &current, force) {
        identity::LockAction::AlreadyPinned(_) => Ok(()),
        identity::LockAction::RefuseWouldReplace(_) | identity::LockAction::RefuseCorrupt => {
            Err(GuiError::IdentityDrift)
        }
        identity::LockAction::Write => {
            identity::write_lock_at(&lock, &current).map_err(|_| GuiError::StorageWriteFailed)
        }
    }
}

/// Decide whether `action` may run for `account`, and in which physical
/// directory (the home directory when none is given). This is the whole
/// policy; a front end only carries the answer to a terminal.
pub fn check_launch(
    config: &AppConfig,
    standard: &Path,
    account: &str,
    action: LaunchAction,
    directory: Option<&str>,
) -> Result<PathBuf, GuiError> {
    match action {
        LaunchAction::Add => {
            if !validate_name(account) || account == "default" {
                return Err(GuiError::InvalidAccount);
            }
            reject_symlinks(&config.base_dir, &config.accounts_dir())?;
            if config.account_path(account).exists() {
                return Err(GuiError::AccountExists);
            }
        }
        LaunchAction::Login => {
            account_dir(config, standard, account)?;
        }
        LaunchAction::Run => {
            let dir = account_dir(config, standard, account)?;
            if matches!(
                identity::lock_state_at(&lock_path(config, account, &dir), &dir),
                identity::LockState::Drift { .. } | identity::LockState::Corrupt
            ) {
                return Err(GuiError::IdentityDrift);
            }
        }
    }
    match directory {
        Some(raw) => project_path(raw, true),
        None => standard
            .parent()
            .map(Path::to_path_buf)
            .ok_or(GuiError::DirectoryMissing),
    }
}

fn writable(config: &AppConfig) -> Result<(), GuiError> {
    for path in [
        config.config_path(),
        config.links_path(),
        config.accounts_dir(),
    ] {
        reject_symlinks(&config.base_dir, &path)?;
    }
    check_readable(&config.config_path())?;
    check_readable(&config.links_path())?;
    config.init().map_err(|_| GuiError::StorageWriteFailed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(tag: &str) -> (AppConfig, PathBuf) {
        let root = std::env::temp_dir()
            .canonicalize()
            .unwrap()
            .join(format!("cc-gui-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let config = AppConfig {
            base_dir: root.join("switch"),
        };
        config.init().unwrap();
        fs::create_dir_all(config.account_path("work")).unwrap();
        (config, root)
    }

    fn pin(config: &AppConfig, name: &str, pinned: &str, signed_in: &str) {
        let dir = config.account_path(name);
        fs::write(
            dir.join(".claude.json"),
            format!(r#"{{"oauthAccount":{{"accountUuid":"{signed_in}"}}}}"#),
        )
        .unwrap();
        fs::write(
            dir.join(identity::LOCK_FILE),
            format!(r#"{{"uuid":"{pinned}"}}"#),
        )
        .unwrap();
    }

    #[test]
    fn snapshot_never_serializes_credentials() {
        let (c, root) = fixture("snapshot");
        let work = c.account_path("work");
        fs::write(
            work.join(".credentials.json"),
            r#"{"accessToken":"SECRET"}"#,
        )
        .unwrap();
        fs::write(
            work.join(".claude.json"),
            r#"{"oauthAccount":{"accountUuid":"a","emailAddress":"owner@example.test"},"secret":"SECRET"}"#,
        )
        .unwrap();
        // The profile cache matches the signed-in uuid, so its plan is shown;
        // its token hash must not travel with it.
        fs::write(
            work.join(".account-info.json"),
            r#"{"uuid":"a","plan":"Pro","token_hash":"HASHSECRET"}"#,
        )
        .unwrap();
        let result = snapshot(&c, &root.join("standard"), false).unwrap();
        assert_eq!(result.accounts[1].plan.as_deref(), Some("Pro"));
        assert_eq!(
            result.accounts[1].email.as_deref(),
            Some("owner@example.test")
        );
        assert_eq!(result.accounts[1].identity_source, IdentitySource::Local);
        let json = serde_json::to_string(&result).unwrap();
        for secret in ["SECRET", "accessToken", "token_hash", ".credentials.json"] {
            assert!(!json.contains(secret), "{secret} leaked into {json}");
        }
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_cached_reading_carries_when_it_was_taken_not_its_age() {
        // Regression: the snapshot sent `age_seconds`, a number frozen at
        // build time, so an open window kept saying "15m ago" for hours.
        let reading = identity::Usage {
            five_hour: Some(identity::UsageWindow {
                utilization: 42.0,
                resets_at: Some("2020-01-01T00:00:00Z".into()),
            }),
            seven_day: None,
        };
        let u = usage(identity::UsageResult::Cached(reading, 900), 1_700_000_900);
        assert_eq!(u.source, UsageSource::Cached);
        assert_eq!(u.fetched_at, Some(1_700_000_000));
        let w = u.five_hour.unwrap();
        // 2020-01-01T00:00:00Z, computed with `date -u -j -f %F%T 2020-01-0100:00:00 +%s`.
        assert_eq!(w.resets_at, Some(1_577_836_800));
        assert!(w.reset_passed(1_577_836_800));
        assert!(!w.reset_passed(1_577_836_799));
        let none = usage(identity::UsageResult::NoToken, 5);
        assert_eq!(none.source, UsageSource::NoCredentials);
        assert_eq!(none.fetched_at, None);
    }

    #[test]
    fn marking_a_default_moves_the_flag_and_keeps_everything_else() {
        let (c, root) = fixture("mark-default");
        let mut s = snapshot(&c, &root.join("standard"), false).unwrap();
        assert_eq!(s.default_account, "default");
        assert!(s.accounts[0].is_default);
        let before = s.accounts[1].usage.clone();
        s.mark_default("work");
        assert_eq!(s.default_account, "work");
        let flags: Vec<_> = s.accounts.iter().map(|a| a.is_default).collect();
        assert_eq!(flags, [false, true]);
        assert_eq!(s.accounts[1].usage, before);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn bindings_share_cli_inheritance_and_allow_missing_entries_to_be_removed() {
        let (c, root) = fixture("binding");
        let standard = root.join("standard");
        let project = root.join("O'Brien 日本 project");
        let child = project.join("nested");
        fs::create_dir_all(&child).unwrap();
        change_binding(&c, &standard, project.to_str().unwrap(), Some("work")).unwrap();
        let seen = bindings(&c, child.to_str()).unwrap().selected.unwrap();
        assert_eq!(seen.account, "work");
        assert_eq!(seen.inherited_from.as_deref(), project.to_str());
        change_binding(&c, &standard, child.to_str().unwrap(), Some("default")).unwrap();
        assert_eq!(
            resolve::resolve_account(&c, &child).as_deref(),
            Some("default")
        );
        fs::remove_dir(&child).unwrap();
        change_binding(&c, &standard, child.to_str().unwrap(), None).unwrap();
        assert_eq!(
            resolve::resolve_account(&c, &child).as_deref(),
            Some("work")
        );
        // Paths are built from the scratch root: "/missing" is not absolute
        // on Windows, which would be refused for the wrong reason.
        let missing = root.join("missing-gui-test-directory");
        assert_eq!(
            change_binding(&c, &standard, missing.to_str().unwrap(), Some("work")),
            Err(GuiError::DirectoryMissing)
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn invalid_names_paths_and_unrequested_defaults_are_rejected() {
        let (c, root) = fixture("validation");
        let standard = root.join("standard");
        assert_eq!(
            account_dir(&c, &standard, "../work"),
            Err(GuiError::InvalidAccount)
        );
        assert_eq!(
            change_default(&c, &standard, "missing"),
            Err(GuiError::AccountMissing)
        );
        assert_eq!(c.get_default().unwrap(), None);
        for raw in ["relative", "/tmp/a\nb", " /tmp/a"] {
            assert_eq!(project_path(raw, false), Err(GuiError::InvalidDirectory));
        }
        // Not from `root`: on Windows that is a canonical `\\?\` verbatim
        // path, where `join("..")` collapses the `..` before it is tested.
        let dotdot = std::env::temp_dir().join("..").join("a");
        assert_eq!(
            project_path(dotdot.to_str().unwrap(), false),
            Err(GuiError::UnrepresentableDirectory)
        );
        change_default(&c, &standard, "work").unwrap();
        assert_eq!(c.get_default().unwrap().as_deref(), Some("work"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn removing_an_account_trashes_it_and_drops_only_what_named_it() {
        let (c, root) = fixture("remove");
        let standard = root.join("standard");
        fs::create_dir_all(c.account_path("other")).unwrap();
        c.set_default("work").unwrap();
        c.set_link("/p/work", "work").unwrap();
        c.set_link("/p/other", "other").unwrap();
        let bin = root.join("bin");
        fs::create_dir_all(&bin).unwrap();

        remove_account_with(&c, &standard, "work", |dir| {
            let dest = bin.join("work");
            fs::rename(dir, &dest).map(|_| dest)
        })
        .unwrap();

        assert!(!c.account_path("work").exists());
        assert!(bin.join("work").is_dir());
        assert_eq!(c.get_default().unwrap(), None);
        assert_eq!(
            c.all_links().unwrap(),
            vec![("/p/other".to_string(), "other".to_string())]
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_failed_move_to_the_trash_changes_nothing() {
        // No fallback to deleting: the window promised the Trash.
        let (c, root) = fixture("remove-fails");
        let standard = root.join("standard");
        c.set_default("work").unwrap();
        c.set_link("/p/work", "work").unwrap();

        let result = remove_account_with(&c, &standard, "work", |_| {
            Err(std::io::Error::from(std::io::ErrorKind::CrossesDevices))
        });

        assert_eq!(result, Err(GuiError::TrashUnavailable));
        assert!(c.account_path("work").is_dir());
        assert_eq!(c.get_default().unwrap().as_deref(), Some("work"));
        assert_eq!(c.all_links().unwrap().len(), 1);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn the_standard_account_and_bad_names_are_never_removed() {
        let (c, root) = fixture("remove-refused");
        let standard = root.join("standard");
        fs::create_dir_all(&standard).unwrap();
        let never = |_: &Path| -> std::io::Result<PathBuf> { panic!("moved") };
        for (name, error) in [
            ("default", GuiError::InvalidAccount),
            ("../work", GuiError::InvalidAccount),
            ("missing", GuiError::AccountMissing),
        ] {
            assert_eq!(
                remove_account_with(&c, &standard, name, never),
                Err(error),
                "{name}"
            );
        }
        assert!(standard.is_dir());
        fs::remove_dir_all(root).unwrap();
    }

    fn signed_in(dir: &Path, uuid: &str) {
        fs::write(
            dir.join(".claude.json"),
            format!(r#"{{"oauthAccount":{{"accountUuid":"{uuid}"}}}}"#),
        )
        .unwrap();
    }

    fn pinned_to(path: &Path) -> Option<String> {
        match identity::read_lock_at(path) {
            identity::LockRead::Pinned(id) => Some(id.uuid),
            _ => None,
        }
    }

    #[test]
    fn pinning_writes_once_and_never_moves_a_pin_without_force() {
        let (c, root) = fixture("pin");
        let standard = root.join("standard");
        let lock = c.account_path("work").join(identity::LOCK_FILE);
        signed_in(&c.account_path("work"), "u-1");
        pin_account(&c, &standard, "work", false).unwrap();
        assert_eq!(pinned_to(&lock).as_deref(), Some("u-1"));
        // Same identity again: nothing to do, not an error.
        pin_account(&c, &standard, "work", false).unwrap();

        // Re-logged in as someone else: that is the drift the pin reports.
        signed_in(&c.account_path("work"), "u-2");
        assert_eq!(
            pin_account(&c, &standard, "work", false),
            Err(GuiError::IdentityDrift)
        );
        assert_eq!(pinned_to(&lock).as_deref(), Some("u-1"));
        pin_account(&c, &standard, "work", true).unwrap();
        assert_eq!(pinned_to(&lock).as_deref(), Some("u-2"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_corrupt_pin_is_replaced_only_with_force() {
        let (c, root) = fixture("pin-corrupt");
        let standard = root.join("standard");
        let lock = c.account_path("work").join(identity::LOCK_FILE);
        signed_in(&c.account_path("work"), "u-1");
        fs::write(&lock, "{").unwrap();
        assert_eq!(
            pin_account(&c, &standard, "work", false),
            Err(GuiError::IdentityDrift)
        );
        assert_eq!(fs::read_to_string(&lock).unwrap(), "{");
        pin_account(&c, &standard, "work", true).unwrap();
        assert_eq!(pinned_to(&lock).as_deref(), Some("u-1"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn an_account_never_signed_in_is_not_pinned_to_nothing() {
        let (c, root) = fixture("pin-none");
        let standard = root.join("standard");
        assert_eq!(
            pin_account(&c, &standard, "work", true),
            Err(GuiError::NotSignedIn)
        );
        assert!(!c.account_path("work").join(identity::LOCK_FILE).exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn the_standard_accounts_pin_goes_beside_our_state_not_into_claude() {
        let (c, root) = fixture("pin-standard");
        let standard = root.join("standard");
        fs::create_dir_all(&standard).unwrap();
        signed_in(&standard, "u-std");
        pin_account(&c, &standard, "default", false).unwrap();
        assert_eq!(
            pinned_to(&identity::default_lock_path(&c.base_dir)).as_deref(),
            Some("u-std")
        );
        assert!(!standard.join(identity::LOCK_FILE).exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn launch_policy_covers_every_action() {
        let (c, root) = fixture("launch");
        let standard = root.join("home").join(".claude");
        fs::create_dir_all(&standard).unwrap();
        let project = root.join("project");
        fs::create_dir(&project).unwrap();
        let run = |name, dir| check_launch(&c, &standard, name, LaunchAction::Run, dir);

        assert_eq!(run("work", None), Ok(root.join("home")));
        assert_eq!(run("work", project.to_str()), Ok(project.clone()));
        assert_eq!(run("default", None), Ok(root.join("home")));
        assert_eq!(run("ghost", None), Err(GuiError::AccountMissing));
        assert_eq!(run("a b", None), Err(GuiError::InvalidAccount));
        let missing = root.join("missing-gui-launch");
        assert_eq!(
            run("work", missing.to_str()),
            Err(GuiError::DirectoryMissing)
        );

        pin(&c, "work", "u-1", "u-1");
        assert_eq!(run("work", None), Ok(root.join("home")));
        pin(&c, "work", "u-1", "u-2");
        assert_eq!(run("work", None), Err(GuiError::IdentityDrift));
        fs::write(c.account_path("work").join(identity::LOCK_FILE), "{").unwrap();
        assert_eq!(run("work", None), Err(GuiError::IdentityDrift));
        // Re-login is how drift gets fixed, so it must stay possible.
        assert!(check_launch(&c, &standard, "work", LaunchAction::Login, None).is_ok());

        let add = |name| check_launch(&c, &standard, name, LaunchAction::Add, None);
        assert!(add("fresh").is_ok());
        assert_eq!(add("work"), Err(GuiError::AccountExists));
        assert_eq!(add("default"), Err(GuiError::InvalidAccount));
        assert_eq!(add("x; rm"), Err(GuiError::InvalidAccount));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn sessions_are_listed_newest_first_with_duplicates_marked() {
        let (c, root) = fixture("sessions");
        fs::create_dir_all(c.account_path("home")).unwrap();
        for (account, id) in [("work", "s-1"), ("home", "s-1"), ("home", "s-2")] {
            let dir = c.account_path(account).join("projects").join("-p");
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join(format!("{id}.jsonl")), "{}\n").unwrap();
        }
        let found = sessions(&c, &root.join("standard")).unwrap();
        let ids: Vec<_> = found
            .iter()
            .map(|s| (s.account.as_str(), s.id.as_str(), s.duplicate))
            .collect();
        assert_eq!(ids.len(), 3);
        assert!(ids.contains(&("work", "s-1", true)));
        assert!(ids.contains(&("home", "s-1", true)));
        assert!(ids.contains(&("home", "s-2", false)));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_live_refresh_is_refused_while_one_credential_store_is_shared() {
        // The only guard against reporting one account's identity and usage
        // for every directory. Decided from the value, so the test needs no
        // process-wide environment variable.
        assert_eq!(live_allowed(true, None), Ok(()));
        assert_eq!(
            live_allowed(true, Some("/Users/x/.claude")),
            Err(GuiError::SharedCredentialStore)
        );
        // Set but empty is the legacy shared entry, not "unset".
        assert_eq!(
            live_allowed(true, Some("")),
            Err(GuiError::SharedCredentialStore)
        );
        assert_eq!(
            live_allowed(true, Some(" ")),
            Err(GuiError::SharedCredentialStore)
        );
        // A local read touches no credential store.
        assert_eq!(live_allowed(false, Some("/Users/x/.claude")), Ok(()));
        assert_eq!(live_allowed(false, None), Ok(()));
    }

    #[test]
    fn corrupt_settings_are_not_replaced_by_a_default_change() {
        let (c, root) = fixture("bad-settings");
        fs::write(c.config_path(), [255, 254]).unwrap();
        assert_eq!(
            change_default(&c, &root.join("standard"), "work"),
            Err(GuiError::StorageUnreadable)
        );
        assert_eq!(fs::read(c.config_path()).unwrap(), vec![255, 254]);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn serialized_shapes_match_the_contract_file() {
        // `gui.contract.json` is the documented wire shape. A consumer can
        // check its own types against the same file, so a rename here has to
        // be a deliberate edit to it, not an accident.
        let raw = include_str!("gui.contract.json");
        let sample: serde_json::Value = serde_json::from_str(raw).unwrap();
        let window = |percent, resets_at| Some(UsageWindow { percent, resets_at });
        let snapshot = Snapshot {
            observed_at: 1_700_000_900,
            default_account: "work".into(),
            accounts: vec![
                Account {
                    name: "default".into(),
                    email: None,
                    plan: None,
                    organization: None,
                    is_default: false,
                    available: false,
                    identity_source: IdentitySource::Unknown,
                    lock: LockStatus::Unknown,
                    usage: Usage {
                        source: UsageSource::Unavailable,
                        fetched_at: None,
                        five_hour: None,
                        seven_day: None,
                    },
                },
                Account {
                    name: "work".into(),
                    email: Some("developer@example.test".into()),
                    plan: Some("Pro".into()),
                    organization: Some("Example Org".into()),
                    is_default: true,
                    available: true,
                    identity_source: IdentitySource::Local,
                    lock: LockStatus::Matched,
                    usage: Usage {
                        source: UsageSource::Cached,
                        fetched_at: Some(1_700_000_000),
                        five_hour: window(42.0, Some(1_700_007_200)),
                        seven_day: window(97.5, Some(1_699_990_000)),
                    },
                },
                Account {
                    name: "personal".into(),
                    email: Some("personal@example.test".into()),
                    plan: None,
                    organization: None,
                    is_default: false,
                    available: true,
                    identity_source: IdentitySource::Live,
                    lock: LockStatus::Drift,
                    usage: Usage {
                        source: UsageSource::Live,
                        fetched_at: Some(1_700_000_900),
                        five_hour: window(5.0, None),
                        seven_day: None,
                    },
                },
            ],
        };
        let bindings = Bindings {
            bindings: vec![Binding {
                directory: "/Projects".into(),
                account: "work".into(),
                exists: true,
            }],
            selected: Some(Selection {
                directory: "/Projects/nested".into(),
                account: "work".into(),
                inherited_from: Some("/Projects".into()),
            }),
        };
        let sessions = vec![Session {
            id: "11111111-2222-3333-4444-555555555555".into(),
            account: "work".into(),
            project: "-Projects".into(),
            modified: Some(1_700_000_000),
            size: 2048,
            duplicate: false,
        }];
        assert_eq!(sample["snapshot"], serde_json::to_value(snapshot).unwrap());
        assert_eq!(sample["bindings"], serde_json::to_value(bindings).unwrap());
        assert_eq!(sample["sessions"], serde_json::to_value(sessions).unwrap());
        let codes = [
            GuiError::InvalidAccount,
            GuiError::AccountMissing,
            GuiError::AccountExists,
            GuiError::SymlinkStorage,
            GuiError::StorageUnreadable,
            GuiError::StorageWriteFailed,
            GuiError::InvalidDirectory,
            GuiError::UnrepresentableDirectory,
            GuiError::DirectoryMissing,
            GuiError::SharedCredentialStore,
            GuiError::IdentityDrift,
            GuiError::TrashUnavailable,
            GuiError::NotSignedIn,
        ];
        assert_eq!(sample["gui_errors"], serde_json::to_value(codes).unwrap());
    }

    // Every test below creates symlinks.
    #[cfg(unix)]
    #[test]
    fn symlinked_storage_is_refused() {
        let (c, root) = fixture("symlink");
        let outside = root.join("outside");
        fs::create_dir(&outside).unwrap();
        std::os::unix::fs::symlink(&outside, c.account_path("alias")).unwrap();
        assert_eq!(
            account_dir(&c, &root.join("standard"), "alias"),
            Err(GuiError::SymlinkStorage)
        );
        std::os::unix::fs::symlink(
            outside.join("private.json"),
            c.account_path("work").join(".claude.json"),
        )
        .unwrap();
        assert_eq!(
            account_dir(&c, &root.join("standard"), "work"),
            Err(GuiError::SymlinkStorage)
        );
        fs::remove_file(c.links_path()).unwrap();
        std::os::unix::fs::symlink(outside.join("links"), c.links_path()).unwrap();
        assert_eq!(
            change_binding(
                &c,
                &root.join("standard"),
                root.to_str().unwrap(),
                Some("default")
            ),
            Err(GuiError::SymlinkStorage)
        );
        assert!(!outside.join("links").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn a_symlink_above_the_storage_is_not_a_symlink_in_it() {
        // Regression: every ancestor up to `/` was checked, so a home reached
        // through a symlink (another volume, /home -> /Users, Nix layouts)
        // refused the whole snapshot as "symlinked account storage".
        let (_, root) = fixture("ancestor");
        let real = root.join("real-home");
        fs::create_dir(&real).unwrap();
        let home = root.join("home");
        std::os::unix::fs::symlink(&real, &home).unwrap();
        let c = AppConfig {
            base_dir: home.join(".claude-switch"),
        };
        c.init().unwrap();
        fs::create_dir_all(c.account_path("work")).unwrap();
        let standard = home.join(".claude");
        fs::create_dir(&standard).unwrap();

        let s = snapshot(&c, &standard, false).unwrap();
        let available: Vec<_> = s
            .accounts
            .iter()
            .map(|a| (a.name.as_str(), a.available))
            .collect();
        assert_eq!(available, [("default", true), ("work", true)]);
        assert!(change_default(&c, &standard, "work").is_ok());
        assert!(bindings(&c, None).is_ok());

        // A symlink planted in that same storage is still refused.
        let outside = root.join("outside");
        fs::create_dir(&outside).unwrap();
        std::os::unix::fs::symlink(
            outside.join("credentials"),
            c.account_path("work").join(".credentials.json"),
        )
        .unwrap();
        assert_eq!(
            account_dir(&c, &standard, "work"),
            Err(GuiError::SymlinkStorage)
        );
        std::os::unix::fs::symlink(&outside, standard.join("projects")).unwrap();
        assert!(sessions(&c, &standard).unwrap().is_empty());
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn project_aliases_resolve_like_the_clis_physical_working_directory() {
        let (c, root) = fixture("project-alias");
        let real = root.join("real");
        fs::create_dir(&real).unwrap();
        let alias = root.join("alias");
        std::os::unix::fs::symlink(&real, &alias).unwrap();
        change_binding(
            &c,
            &root.join("standard"),
            alias.to_str().unwrap(),
            Some("work"),
        )
        .unwrap();
        assert_eq!(
            c.get_link(real.to_str().unwrap()).unwrap().as_deref(),
            Some("work")
        );
        assert_eq!(
            bindings(&c, alias.to_str())
                .unwrap()
                .selected
                .unwrap()
                .directory,
            real.to_str().unwrap()
        );
        fs::remove_dir_all(root).unwrap();
    }
}
