use std::path::PathBuf;

use crate::config::{AppConfig, is_reserved_name, validate_name};
use crate::i18n::{I18n, Msg};
use crate::identity::{self, Identity, LockState};

/// Where an account's pin and its config dir live.
///
/// The standard account is the odd one out on both counts: its config dir is
/// `~/.claude/`, which belongs to Claude Code and which we only read, so its
/// pin goes beside our own state instead — the same rule the doctor cache
/// already follows.
pub fn paths(config: &AppConfig, name: &str) -> Option<(PathBuf, PathBuf)> {
    if name == "default" {
        let config_dir = identity::standard_token_dir()?;
        return Some((identity::default_lock_path(&config.base_dir), config_dir));
    }
    let acc = config.account_path(name);
    Some((acc.join(identity::LOCK_FILE), acc))
}

/// Pin an account to the identity it is signed in as now, so a later re-login
/// as someone else shows up as drift instead of passing unnoticed.
pub fn run(config: &AppConfig, i18n: &I18n, name: &str, force: bool) -> i32 {
    if name != "default" {
        if is_reserved_name(name) {
            i18n.print(Msg::ReservedName(name.to_string()));
            return 1;
        }
        if !validate_name(name) {
            i18n.print(Msg::NameInvalid);
            return 1;
        }
        if !config.account_path(name).is_dir() {
            i18n.print(Msg::LoginNotFound(name.to_string()));
            return 1;
        }
    }

    let Some((lock_path, config_dir)) = paths(config, name) else {
        i18n.print(Msg::LoginNotFound(name.to_string()));
        return 1;
    };

    let Some(current) = identity::local_identity(&config_dir) else {
        // Nothing to pin to. Claude Code writes this file when it signs in,
        // so its absence means this dir has never been logged in — pinning
        // it to nothing would be a lock nobody can satisfy.
        i18n.print(Msg::LockNoIdentity(name.to_string()));
        return 1;
    };

    // An existing pin is not replaced silently: re-pinning is how a drift
    // warning gets switched off, and doing that by accident is the one
    // outcome this command must not produce.
    if let Some(existing) = identity::read_lock_at(&lock_path)
        && !force
    {
        if existing.uuid == current.uuid {
            i18n.print(Msg::LockAlready(name.to_string(), describe(&existing)));
            return 0;
        }
        i18n.print(Msg::LockWouldReplace(
            name.to_string(),
            describe(&existing),
            describe(&current),
        ));
        return 1;
    }

    if let Err(e) = identity::write_lock_at(&lock_path, &current) {
        i18n.print(Msg::LockWriteFailed(e.to_string()));
        return 1;
    }
    i18n.print(Msg::LockDone(name.to_string(), describe(&current)));
    0
}

/// Pin an account after a login, without failing anything if it does not
/// work — the login is what mattered, and `lock` can always be run by hand.
///
/// Only writes when there is no pin yet. Re-logging in to an account that is
/// already pinned must not move the pin: that is exactly the drift the pin
/// exists to report.
pub fn write_after_login(config: &AppConfig, name: &str) {
    let Some((lock_path, config_dir)) = paths(config, name) else {
        return;
    };
    if identity::read_lock_at(&lock_path).is_some() {
        return;
    }
    if let Some(current) = identity::local_identity(&config_dir) {
        let _ = identity::write_lock_at(&lock_path, &current);
    }
}

/// `email (uuid)`, or just the uuid when the pin predates an email being
/// recorded.
pub fn describe(id: &Identity) -> String {
    match &id.email {
        Some(email) => format!("{} ({})", email, id.uuid),
        None => id.uuid.clone(),
    }
}

/// The one-line marker `doctor` appends to an account's row.
pub fn state_marker(config: &AppConfig, name: &str, i18n: &I18n) -> String {
    let Some((lock_path, config_dir)) = paths(config, name) else {
        return String::new();
    };
    match identity::lock_state_at(&lock_path, &config_dir) {
        // Silence is the right report for a pin that holds: doctor's job is
        // to surface what needs attention, and a row that says "fine" next to
        // every other row says nothing at all.
        LockState::Ok | LockState::NoLock => String::new(),
        LockState::Unknown => format!("  {}", i18n.msg(Msg::DoctorLockUnknown)),
        LockState::Drift { expected, actual } => format!(
            "  {}",
            i18n.msg(Msg::DoctorLockDrift(describe(&expected), describe(&actual)))
        ),
    }
}

/// Whether any account is showing drift — `doctor` exits non-zero on that, so
/// a script can gate on it.
pub fn any_drift(config: &AppConfig, names: &[String]) -> bool {
    names.iter().any(|name| {
        paths(config, name)
            .map(|(lock, dir)| {
                matches!(
                    identity::lock_state_at(&lock, &dir),
                    LockState::Drift { .. }
                )
            })
            .unwrap_or(false)
    })
}
