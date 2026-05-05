use crate::config::AppConfig;
use crate::i18n::{I18n, Msg};
use crate::identity::{self, AuditResult};

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
    let mut healthy = 0usize;

    for acc in accounts {
        let acc_dir = config.account_path(acc);
        let pad = " ".repeat(label_w.saturating_sub(acc.len()));
        match identity::audit_account(&acc_dir) {
            AuditResult::Ok(p) => {
                healthy += 1;
                let email = p.email.as_deref().unwrap_or("<unknown>");
                let uuid = p.uuid.as_deref().unwrap_or("<unknown>");
                println!("  ✓ {}{}  {}  uuid={}", acc, pad, email, uuid);
            }
            AuditResult::NoToken => {
                println!(
                    "  ? {}{}  {}",
                    acc,
                    pad,
                    i18n.msg(Msg::DoctorNoToken(acc.clone()))
                );
            }
            AuditResult::Offline => {
                println!("  ? {}{}  {}", acc, pad, i18n.msg(Msg::DoctorOffline));
            }
        }
    }

    if standard_present {
        let pad = " ".repeat(label_w.saturating_sub(standard_label.len()));
        match identity::audit_default(&config.base_dir) {
            AuditResult::Ok(p) => {
                healthy += 1;
                let email = p.email.as_deref().unwrap_or("<unknown>");
                let uuid = p.uuid.as_deref().unwrap_or("<unknown>");
                println!(
                    "  ✓ {}{}  {}  uuid={}  ({})",
                    standard_label,
                    pad,
                    email,
                    uuid,
                    i18n.msg(Msg::ListStandard)
                );
            }
            AuditResult::Offline => {
                println!(
                    "  ? {}{}  {}",
                    standard_label,
                    pad,
                    i18n.msg(Msg::DoctorOffline)
                );
            }
            AuditResult::NoToken => {
                // standard_present was true above, but the token could
                // have disappeared between the two reads — fall through.
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
///     {"name": "work", "status": "ok", "email": "...", "uuid": "...", "default": true},
///     {"name": "personal", "status": "no_token", "email": null, "uuid": null, "default": false}
///   ],
///   "standard": {"status": "ok", "email": "...", "uuid": "..."} | null
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
    let (status, email, uuid) = match result {
        AuditResult::Ok(p) => ("ok", p.email, p.uuid),
        AuditResult::NoToken => ("no_token", None, None),
        AuditResult::Offline => ("offline", None, None),
    };
    let mut obj = serde_json::json!({
        "name": name,
        "status": status,
        "email": email,
        "uuid": uuid,
    });
    if let Some(d) = is_default {
        obj["default"] = serde_json::Value::Bool(d);
    }
    obj
}
