use std::collections::HashMap;

use crate::config::AppConfig;
use crate::i18n::{I18n, Msg};
use crate::identity::{self, AuditResult, Profile};

/// "  Max 20x" when the profile carries a plan, else "". Rendered right after
/// the email in the human audit output.
fn plan_seg(p: &Profile) -> String {
    p.plan
        .as_deref()
        .map(|s| format!("  {}", s))
        .unwrap_or_default()
}

/// "  ↔ same identity as work, personal" when other audited accounts resolve to
/// the same UUID, else "". Sharing one login across dirs is a legitimate setup
/// (e.g. to keep separate global settings / plugins under one subscription), so
/// this is a neutral cross-reference, not a warning.
fn shared_seg(
    uuid: Option<&str>,
    label: &str,
    by_uuid: &HashMap<&str, Vec<&str>>,
    i18n: &I18n,
) -> String {
    let Some(uuid) = uuid else {
        return String::new();
    };
    let others: Vec<&str> = by_uuid
        .get(uuid)
        .map(|labels| labels.iter().copied().filter(|l| *l != label).collect())
        .unwrap_or_default();
    if others.is_empty() {
        return String::new();
    }
    format!(
        "  {}",
        i18n.msg(Msg::DoctorSharedIdentity(others.join(", ")))
    )
}

pub fn run(config: &AppConfig, i18n: &I18n, json: bool) -> i32 {
    let accounts = match config.list_accounts() {
        Ok(v) => v,
        Err(_) => return 1,
    };

    // The standard "default" line shows up if a login exists in ~/.claude/.
    // Otherwise it would just print noise on every doctor run, so we hide it
    // when there's no token there.
    let standard_present = identity::standard_token_dir()
        .map(|d| identity::current_token_hash(&d).is_some())
        .unwrap_or(false);

    if json {
        return run_json(config, &accounts, standard_present);
    }
    run_human(config, i18n, &accounts, standard_present)
}

fn run_human(config: &AppConfig, i18n: &I18n, accounts: &[String], standard_present: bool) -> i32 {
    let standard_label = "~/.claude/";

    // The standard account earns a row when it has a token *or* when its pin
    // has something to say. Gating the pin check on the token would hide
    // drift in `~/.claude.json` completely — and that account is the one
    // `claude` falls back to, so a wrong identity there is the easiest to
    // walk into. Computed before the empty-check below for the same reason.
    let standard_lock_finding = !super::lock::state_marker(config, "default", i18n).is_empty();

    if accounts.is_empty() && !standard_present && !standard_lock_finding {
        i18n.print(Msg::ListEmpty);
        return 0;
    }

    let total = accounts.len()
        + if standard_present || standard_lock_finding {
            1
        } else {
            0
        };
    i18n.print(Msg::DoctorHeader(total));

    let label_w = accounts
        .iter()
        .map(|a| a.len())
        .chain(std::iter::once(if standard_present {
            standard_label.len()
        } else {
            0
        }))
        .max()
        .unwrap_or(0);

    // Audit everything up front so we can cross-reference identities before
    // printing (the shared-identity note needs all UUIDs in hand).
    let mut rows: Vec<(String, AuditResult)> = accounts
        .iter()
        .map(|acc| {
            (
                acc.clone(),
                identity::audit_account(&config.account_path(acc)),
            )
        })
        .collect();
    if standard_present || standard_lock_finding {
        rows.push((
            standard_label.to_string(),
            identity::audit_default(&config.base_dir),
        ));
    }

    // UUID -> every audited label resolving to it, for the shared-identity note.
    let mut by_uuid: HashMap<&str, Vec<&str>> = HashMap::new();
    for (label, res) in &rows {
        if let AuditResult::Ok(p) = res
            && let Some(uuid) = p.uuid.as_deref()
        {
            by_uuid.entry(uuid).or_default().push(label.as_str());
        }
    }

    let mut healthy = 0usize;
    let mut drift = false;
    for (label, res) in &rows {
        let is_standard = label == standard_label;
        let pad = " ".repeat(label_w.saturating_sub(label.len()));
        let acc_name = if is_standard {
            "default"
        } else {
            label.as_str()
        };

        // The pin comparison reads Claude Code's own local record — a small
        // file, no keychain and no network — so unlike the rest of this audit
        // it still works when the account is offline or has no token. Compute
        // it for every row, not just the healthy ones: an account that cannot
        // reach the API is exactly when a silent wrong-identity would hurt
        // most, and a hint printed with no row to point at is worse than
        // useless.
        let lock = super::lock::state_marker(config, acc_name, i18n);
        drift |= super::lock::is_drift(config, acc_name);

        match res {
            AuditResult::Ok(p) => {
                healthy += 1;
                let email = p.email.as_deref().unwrap_or("<unknown>");
                let uuid = p.uuid.as_deref().unwrap_or("<unknown>");
                let shared = shared_seg(p.uuid.as_deref(), label, &by_uuid, i18n);
                if is_standard {
                    println!(
                        "  ✓ {}{}  {}{}  uuid={}  {}{}{}",
                        label,
                        pad,
                        email,
                        plan_seg(p),
                        uuid,
                        i18n.msg(Msg::ListStandard),
                        shared,
                        lock
                    );
                } else {
                    println!(
                        "  ✓ {}{}  {}{}  uuid={}{}{}",
                        label,
                        pad,
                        email,
                        plan_seg(p),
                        uuid,
                        shared,
                        lock
                    );
                }
            }
            AuditResult::Offline => {
                println!(
                    "  ? {}{}  {}{}",
                    label,
                    pad,
                    i18n.msg(Msg::DoctorOffline),
                    lock
                );
            }
            AuditResult::NoToken => {
                // For the standard row this normally just means the token
                // vanished between the presence check and the audit — skip it
                // silently, matching the prior behaviour. Unless the pin has
                // something to report, in which case swallowing the row would
                // leave the hint below with nothing to point at.
                if !is_standard || !lock.is_empty() {
                    println!(
                        "  ? {}{}  {}{}",
                        label,
                        pad,
                        // The hint names the argument `login` actually takes.
                        // For the standard account that is `default`, not the
                        // `~/.claude/` label the row is titled with.
                        i18n.msg(Msg::DoctorNoToken(acc_name.to_string())),
                        lock
                    );
                }
            }
        }
    }

    // Drift is the one thing here that means "you are about to do work under
    // the wrong account", so it decides the exit code even when every account
    // audited fine. It is accumulated from the rows actually printed, so the
    // hint below can never appear without a row explaining it.
    println!();
    if drift {
        i18n.print(Msg::DoctorDriftHint);
    }
    // Exactly one summary line, always — a script grepping for either of
    // these must not find silence just because drift showed up alongside a
    // clean audit.
    if healthy == total {
        i18n.print(Msg::DoctorAllOk);
    } else {
        i18n.print(Msg::DoctorPartial(healthy, total));
    }
    if healthy == total && !drift { 0 } else { 1 }
}

/// Emit the same audit information as `run_human`, but as a single JSON
/// document on stdout — for scripting. Schema:
///
/// ```json
/// {
///   "accounts": [
///     {"name": "work", "status": "ok", "email": "...", "uuid": "...", "plan": "Max 20x", "default": true},
///     {"name": "personal", "status": "no_token", "email": null, "uuid": null, "plan": null, "default": false}
///   ],
///   "standard": {"status": "ok", "email": "...", "uuid": "...", "plan": "..."} | null
/// }
/// ```
///
/// Each entry also carries `"lock"`: `"ok"`, `"drift"`, `"none"`, `"unknown"`
/// or `"corrupt"`, plus `"pinned_uuid"` when there is a pin. The human form
/// exits 1 on drift and says why; a `--json` consumer is the one most likely
/// to be gating a script on this, so it must be able to see the same thing —
/// reporting it only to the human would leave automation blind to exactly
/// what this is for.
///
/// Same exit semantics as the human form: 0 if all audited entries are `ok`
/// and nothing has drifted, 1 otherwise. `no_token` entries are *not* counted
/// as failures (an account that hasn't been logged into is a known-empty
/// state, not an error).
fn run_json(config: &AppConfig, accounts: &[String], standard_present: bool) -> i32 {
    let default_acc = config.get_default().ok().flatten();
    let mut entries = Vec::with_capacity(accounts.len());
    let mut any_problem = false;

    for acc in accounts {
        let acc_dir = config.account_path(acc);
        let mut entry = build_entry(
            acc.as_str(),
            identity::audit_account(&acc_dir),
            Some(default_acc.as_deref() == Some(acc.as_str())),
        );
        add_lock_fields(config, acc.as_str(), &mut entry);
        if entry["status"] == "offline" || entry["lock"] == "drift" {
            any_problem = true;
        }
        entries.push(entry);
    }

    // The standard account's pin is checked whether or not it has a token:
    // the comparison reads a local file, and an account that cannot be
    // audited is not a reason to stop reporting that it is the wrong one.
    let standard = if standard_present {
        let mut entry = build_entry(
            "~/.claude/",
            identity::audit_default(&config.base_dir),
            None,
        );
        add_lock_fields(config, "default", &mut entry);
        if entry["status"] == "offline" || entry["lock"] == "drift" {
            any_problem = true;
        }
        Some(entry)
    } else if super::lock::json_state(config, "default").0 != "none" {
        // Same rule as the human form: a pin with something to report gets an
        // entry even when the account has no token to audit.
        let mut entry = build_entry("~/.claude/", AuditResult::NoToken, None);
        add_lock_fields(config, "default", &mut entry);
        if entry["lock"] == "drift" {
            any_problem = true;
        }
        Some(entry)
    } else {
        None
    };

    let doc = serde_json::json!({
        "accounts": entries,
        "standard": standard,
    });
    println!("{}", serde_json::to_string_pretty(&doc).unwrap_or_default());

    if any_problem { 1 } else { 0 }
}

/// Add `lock` and `pinned_uuid` to an entry. Separate from `build_entry`
/// because the pin is read from a local file rather than from the audit —
/// different source, different failure modes.
fn add_lock_fields(config: &AppConfig, acc_name: &str, entry: &mut serde_json::Value) {
    let (state, pinned) = super::lock::json_state(config, acc_name);
    if let Some(obj) = entry.as_object_mut() {
        obj.insert("lock".to_string(), serde_json::json!(state));
        obj.insert("pinned_uuid".to_string(), serde_json::json!(pinned));
    }
}

fn build_entry(name: &str, result: AuditResult, is_default: Option<bool>) -> serde_json::Value {
    let (status, email, uuid, plan) = match result {
        AuditResult::Ok(p) => ("ok", p.email, p.uuid, p.plan),
        AuditResult::NoToken => ("no_token", None, None, None),
        AuditResult::Offline => ("offline", None, None, None),
    };
    let mut obj = serde_json::json!({
        "name": name,
        "status": status,
        "email": email,
        "uuid": uuid,
        "plan": plan,
    });
    if let Some(d) = is_default {
        obj["default"] = serde_json::Value::Bool(d);
    }
    obj
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::Lang;

    fn en() -> I18n {
        I18n { lang: Lang::En }
    }

    #[test]
    fn shared_seg_empty_when_identity_unique() {
        let mut by_uuid = HashMap::new();
        by_uuid.insert("u1", vec!["work"]);
        assert_eq!(shared_seg(Some("u1"), "work", &by_uuid, &en()), "");
    }

    #[test]
    fn shared_seg_lists_the_other_accounts() {
        let mut by_uuid = HashMap::new();
        by_uuid.insert("u1", vec!["work", "settings", "personal"]);
        assert_eq!(
            shared_seg(Some("u1"), "work", &by_uuid, &en()),
            "  ↔ same identity as settings, personal"
        );
    }

    #[test]
    fn shared_seg_empty_without_uuid() {
        let by_uuid = HashMap::new();
        assert_eq!(shared_seg(None, "work", &by_uuid, &en()), "");
    }
}
