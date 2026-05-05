use crate::config::AppConfig;
use crate::i18n::{I18n, Msg};
use crate::identity::{self, AuditResult};

pub fn run(config: &AppConfig, i18n: &I18n) -> i32 {
    let accounts = match config.list_accounts() {
        Ok(v) => v,
        Err(_) => return 1,
    };

    // The standard "default" line shows up if a login exists in ~/.claude/.
    // Otherwise it would just print noise on every doctor run, so we hide it
    // when there's no token there.
    let standard_label = "~/.claude/";
    let standard_present = identity::standard_token_dir()
        .map(|d| identity::current_token_hash(&d).is_some())
        .unwrap_or(false);

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

    for acc in &accounts {
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
