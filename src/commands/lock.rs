use std::path::PathBuf;

use crate::config::{AppConfig, is_reserved_name, validate_name};
use crate::i18n::{I18n, Msg};
use crate::identity::{self, Identity, LockRead, LockState};

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

    match plan_lock(&identity::read_lock_at(&lock_path), &current, force) {
        LockAction::AlreadyPinned(existing) => {
            i18n.print(Msg::LockAlready(name.to_string(), describe(&existing)));
            return 0;
        }
        LockAction::RefuseWouldReplace(existing) => {
            i18n.print(Msg::LockWouldReplace(
                name.to_string(),
                describe(&existing),
                describe(&current),
            ));
            return 1;
        }
        LockAction::RefuseCorrupt => {
            i18n.print(Msg::LockCorrupt(
                name.to_string(),
                lock_path.display().to_string(),
                name.to_string(),
            ));
            return 1;
        }
        LockAction::Write => {}
    }

    if let Err(e) = identity::write_lock_at(&lock_path, &current) {
        i18n.print(Msg::LockWriteFailed(e.to_string()));
        return 1;
    }
    i18n.print(Msg::LockDone(name.to_string(), describe(&current)));
    0
}

/// What `lock` does with one account, given what its pin file says.
///
/// Split out from the command the way `plan_install`/`plan_uninstall` are in
/// `commands/vscode.rs`: the policy here — never replace a pin by accident —
/// is the whole point of the feature, and tangled with printing it could not
/// be checked at all.
#[derive(Debug, PartialEq)]
pub enum LockAction {
    Write,
    AlreadyPinned(Identity),
    /// Pinned to somebody else. This *is* drift; replacing it takes `--force`,
    /// because re-pinning is how the warning gets switched off.
    RefuseWouldReplace(Identity),
    /// A pin that cannot be read counts as existing. Treating it as absent
    /// would let a corrupted file turn the protection off and report success.
    RefuseCorrupt,
}

pub fn plan_lock(existing: &LockRead, current: &Identity, force: bool) -> LockAction {
    if force {
        return LockAction::Write;
    }
    match existing {
        LockRead::None => LockAction::Write,
        LockRead::Corrupt => LockAction::RefuseCorrupt,
        LockRead::Pinned(p) if p.uuid == current.uuid => LockAction::AlreadyPinned(p.clone()),
        LockRead::Pinned(p) => LockAction::RefuseWouldReplace(p.clone()),
    }
}

/// Whether an automatic pin after login should write. Only "no pin at all"
/// qualifies: re-logging in to a pinned account must not move the pin, and a
/// corrupt one is not ours to quietly replace either.
pub fn should_pin_after_login(existing: &LockRead) -> bool {
    matches!(existing, LockRead::None)
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
    if !should_pin_after_login(&identity::read_lock_at(&lock_path)) {
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
        LockState::Corrupt => format!("  {}", i18n.msg(Msg::DoctorLockCorrupt)),
        LockState::Drift { expected, actual } => format!(
            "  {}",
            i18n.msg(Msg::DoctorLockDrift(describe(&expected), describe(&actual)))
        ),
    }
}

/// The pin state as a stable string for `--json`, plus the pinned uuid when
/// there is one. Kept next to `state_marker` so the human and machine views
/// cannot drift apart themselves.
pub fn json_state(config: &AppConfig, name: &str) -> (&'static str, Option<String>) {
    let Some((lock_path, config_dir)) = paths(config, name) else {
        return ("none", None);
    };
    let pinned = match identity::read_lock_at(&lock_path) {
        LockRead::Pinned(id) => Some(id.uuid),
        _ => None,
    };
    let state = match identity::lock_state_at(&lock_path, &config_dir) {
        LockState::Ok => "ok",
        LockState::Drift { .. } => "drift",
        LockState::NoLock => "none",
        LockState::Unknown => "unknown",
        LockState::Corrupt => "corrupt",
    };
    (state, pinned)
}

/// Whether this account is showing drift — `doctor` exits non-zero when any
/// is, so a script can gate on it.
pub fn is_drift(config: &AppConfig, name: &str) -> bool {
    paths(config, name)
        .map(|(lock, dir)| {
            matches!(
                identity::lock_state_at(&lock, &dir),
                LockState::Drift { .. }
            )
        })
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn id(uuid: &str, email: Option<&str>) -> Identity {
        Identity {
            uuid: uuid.to_string(),
            email: email.map(String::from),
        }
    }

    fn scratch(what: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("cc-lockcmd-{}-{}", what, std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn pinning_an_unpinned_account_writes_and_pinning_the_same_one_twice_does_not() {
        let now = id("u-1", Some("a@example.com"));
        assert_eq!(plan_lock(&LockRead::None, &now, false), LockAction::Write);
        assert_eq!(
            plan_lock(&LockRead::Pinned(now.clone()), &now, false),
            LockAction::AlreadyPinned(now)
        );
    }

    #[test]
    fn a_pin_belonging_to_another_account_is_never_replaced_without_force() {
        // The single invariant the feature exists for: a re-login must not be
        // able to move a pin, because that swap is the drift being reported.
        let pinned = id("u-work", Some("work@example.com"));
        let now = id("u-personal", Some("personal@example.com"));
        assert_eq!(
            plan_lock(&LockRead::Pinned(pinned.clone()), &now, false),
            LockAction::RefuseWouldReplace(pinned.clone())
        );
        // And --force is the deliberate override.
        assert_eq!(
            plan_lock(&LockRead::Pinned(pinned), &now, true),
            LockAction::Write
        );
    }

    #[test]
    fn a_corrupt_pin_is_refused_rather_than_treated_as_absent() {
        // Regression: every read failure used to collapse to "no pin", so a
        // truncated file let `lock` overwrite it and report success — the
        // protection switching itself off.
        let now = id("u-1", None);
        assert_eq!(
            plan_lock(&LockRead::Corrupt, &now, false),
            LockAction::RefuseCorrupt
        );
        assert_eq!(plan_lock(&LockRead::Corrupt, &now, true), LockAction::Write);
    }

    #[test]
    fn only_an_account_with_no_pin_at_all_is_pinned_automatically_after_a_login() {
        // `login` runs this unattended. Anything already on disk — a pin that
        // matches, one that does not, or one that cannot be read — is left
        // exactly as it is.
        assert!(should_pin_after_login(&LockRead::None));
        assert!(!should_pin_after_login(&LockRead::Pinned(id("u-1", None))));
        assert!(!should_pin_after_login(&LockRead::Corrupt));
    }

    #[test]
    fn the_automatic_pin_does_not_touch_a_file_that_is_already_there() {
        // The decision above is only half of it — this checks the bytes.
        let dir = scratch("afterlogin");
        let config = AppConfig {
            base_dir: dir.clone(),
        };
        let acc = config.account_path("work");
        fs::create_dir_all(&acc).unwrap();
        fs::write(
            acc.join(".claude.json"),
            serde_json::json!({"oauthAccount": {"accountUuid": "u-new"}}).to_string(),
        )
        .unwrap();

        // No pin yet: one is written, recording who signed in.
        write_after_login(&config, "work");
        let written = fs::read_to_string(acc.join(identity::LOCK_FILE)).unwrap();
        assert!(written.contains("u-new"), "{written}");

        // Signed in as somebody else now — the pin must not follow.
        fs::write(
            acc.join(".claude.json"),
            serde_json::json!({"oauthAccount": {"accountUuid": "u-other"}}).to_string(),
        )
        .unwrap();
        write_after_login(&config, "work");
        assert_eq!(
            fs::read_to_string(acc.join(identity::LOCK_FILE)).unwrap(),
            written,
            "the pin moved on a re-login"
        );

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_standard_accounts_pin_is_kept_out_of_the_claude_directory() {
        // `~/.claude/` belongs to Claude Code; we read it and do not write to
        // it. Its pin goes beside our own state, like the doctor cache.
        let config = AppConfig {
            base_dir: std::path::PathBuf::from("/tmp/switch"),
        };
        let (lock, config_dir) = paths(&config, "default").unwrap();
        assert_eq!(
            lock,
            std::path::PathBuf::from("/tmp/switch/default.identity-lock.json")
        );
        assert_eq!(config_dir, identity::standard_token_dir().unwrap());
        assert!(!lock.starts_with(&config_dir));

        let (lock, config_dir) = paths(&config, "work").unwrap();
        assert_eq!(config_dir, config.account_path("work"));
        assert_eq!(lock, config_dir.join(identity::LOCK_FILE));
    }

    #[test]
    fn doctor_reports_drift_only_for_the_account_that_has_it() {
        let dir = scratch("isdrift");
        let config = AppConfig {
            base_dir: dir.clone(),
        };
        for (name, pinned, signed_in) in [
            ("clean", Some("u-1"), "u-1"),
            ("drifted", Some("u-1"), "u-2"),
            ("unpinned", None, "u-3"),
        ] {
            let acc = config.account_path(name);
            fs::create_dir_all(&acc).unwrap();
            fs::write(
                acc.join(".claude.json"),
                serde_json::json!({"oauthAccount": {"accountUuid": signed_in}}).to_string(),
            )
            .unwrap();
            if let Some(uuid) = pinned {
                identity::write_lock_at(&acc.join(identity::LOCK_FILE), &id(uuid, None)).unwrap();
            }
        }

        assert!(is_drift(&config, "drifted"));
        // The do-nothing branches: neither a matching pin nor no pin at all
        // may fail a doctor run.
        assert!(!is_drift(&config, "clean"));
        assert!(!is_drift(&config, "unpinned"));

        assert_eq!(json_state(&config, "drifted").0, "drift");
        assert_eq!(json_state(&config, "clean").0, "ok");
        assert_eq!(json_state(&config, "unpinned").0, "none");
        assert_eq!(json_state(&config, "drifted").1.as_deref(), Some("u-1"));

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_marker_is_silent_when_the_pin_holds_and_names_both_sides_when_it_does_not() {
        let dir = scratch("marker");
        let config = AppConfig {
            base_dir: dir.clone(),
        };
        let i18n = I18n {
            lang: crate::i18n::Lang::En,
        };

        let acc = config.account_path("work");
        fs::create_dir_all(&acc).unwrap();
        fs::write(
            acc.join(".claude.json"),
            serde_json::json!({"oauthAccount":
                {"accountUuid": "u-live", "emailAddress": "live@example.com"}})
            .to_string(),
        )
        .unwrap();

        // No pin: nothing to say. Every account predating this feature is
        // here, so noise would bury the real finding.
        assert_eq!(state_marker(&config, "work", &i18n), "");

        identity::write_lock_at(
            &acc.join(identity::LOCK_FILE),
            &id("u-live", Some("live@example.com")),
        )
        .unwrap();
        assert_eq!(state_marker(&config, "work", &i18n), "");

        identity::write_lock_at(
            &acc.join(identity::LOCK_FILE),
            &id("u-pinned", Some("pinned@example.com")),
        )
        .unwrap();
        let marker = state_marker(&config, "work", &i18n);
        // Both identities, or the reader cannot tell which way the swap went.
        assert!(marker.contains("pinned@example.com"), "{marker}");
        assert!(marker.contains("live@example.com"), "{marker}");
        assert!(
            marker.contains("u-pinned") && marker.contains("u-live"),
            "{marker}"
        );

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_pin_filenames_match_what_the_shell_implementation_writes() {
        // These names are a cross-implementation contract, not an internal
        // detail: `claude-switch.sh` hardcodes both strings. Every other test
        // here refers to them through the constants, so renaming one would
        // rename it consistently everywhere and pass — while the shell port
        // silently stopped seeing the same files.
        assert_eq!(identity::LOCK_FILE, ".identity-lock.json");
        let config = AppConfig {
            base_dir: std::path::PathBuf::from("/tmp/switch"),
        };
        assert_eq!(
            paths(&config, "default").unwrap().0.file_name().unwrap(),
            "default.identity-lock.json"
        );
    }

    #[test]
    fn an_identity_is_described_with_its_email_when_it_has_one() {
        assert_eq!(
            describe(&id("u-1", Some("a@example.com"))),
            "a@example.com (u-1)"
        );
        assert_eq!(describe(&id("u-1", None)), "u-1");
    }
}
