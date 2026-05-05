use crate::config::AppConfig;
use crate::i18n::{I18n, Msg};
use crate::identity::{self, AuditResult};

pub fn run(config: &AppConfig, i18n: &I18n) -> i32 {
    let accounts = match config.list_accounts() {
        Ok(v) => v,
        Err(_) => return 1,
    };
    if accounts.is_empty() {
        i18n.print(Msg::ListEmpty);
        return 0;
    }

    i18n.print(Msg::DoctorHeader(accounts.len()));

    let label_w = accounts.iter().map(|a| a.len()).max().unwrap_or(0);
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

    println!();
    if healthy == accounts.len() {
        i18n.print(Msg::DoctorAllOk);
        0
    } else {
        i18n.print(Msg::DoctorPartial(healthy, accounts.len()));
        1
    }
}
