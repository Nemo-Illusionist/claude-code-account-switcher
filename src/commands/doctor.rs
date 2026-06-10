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

    if accounts.is_empty() && !standard_present {
        i18n.print(Msg::ListEmpty);
        return 0;
    }

    let total = accounts.len() + if standard_present { 1 } else { 0 };
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
    if standard_present {
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
    for (label, res) in &rows {
        let is_standard = label == standard_label;
        let pad = " ".repeat(label_w.saturating_sub(label.len()));
        match res {
            AuditResult::Ok(p) => {
                healthy += 1;
                let email = p.email.as_deref().unwrap_or("<unknown>");
                let uuid = p.uuid.as_deref().unwrap_or("<unknown>");
                let shared = shared_seg(p.uuid.as_deref(), label, &by_uuid, i18n);
                if is_standard {
                    println!(
                        "  ✓ {}{}  {}{}  uuid={}  ({}){}",
                        label,
                        pad,
                        email,
                        plan_seg(p),
                        uuid,
                        i18n.msg(Msg::ListStandard),
                        shared
                    );
                } else {
                    println!(
                        "  ✓ {}{}  {}{}  uuid={}{}",
                        label,
                        pad,
                        email,
                        plan_seg(p),
                        uuid,
                        shared
                    );
                }
            }
            AuditResult::Offline => {
                println!("  ? {}{}  {}", label, pad, i18n.msg(Msg::DoctorOffline));
            }
            AuditResult::NoToken => {
                // For the standard row this just means the token vanished
                // between the presence check and the audit — skip it silently,
                // matching the prior behavior.
                if !is_standard {
                    println!(
                        "  ? {}{}  {}",
                        label,
                        pad,
                        i18n.msg(Msg::DoctorNoToken(label.clone()))
                    );
                }
            }
        }
    }

    println!();
    if healthy == total {
        i18n.print(Msg::DoctorAllOk);
        0
    } else {
        i18n.print(Msg::DoctorPartial(healthy, total));
        1
    }
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
/// Same exit semantics as the human form: 0 if all audited entries are `ok`,
/// 1 otherwise. `no_token` entries are *not* counted as failures (an account
/// that hasn't been logged into is a known-empty state, not an error).
fn run_json(config: &AppConfig, accounts: &[String], standard_present: bool) -> i32 {
    let default_acc = config.get_default().ok().flatten();
    let mut entries = Vec::with_capacity(accounts.len());
    let mut any_problem = false;

    for acc in accounts {
        let acc_dir = config.account_path(acc);
        let entry = build_entry(
            acc.as_str(),
            identity::audit_account(&acc_dir),
            Some(default_acc.as_deref() == Some(acc.as_str())),
        );
        if entry["status"] == "offline" {
            any_problem = true;
        }
        entries.push(entry);
    }

    let standard = if standard_present {
        let entry = build_entry(
            "~/.claude/",
            identity::audit_default(&config.base_dir),
            None,
        );
        if entry["status"] == "offline" {
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
