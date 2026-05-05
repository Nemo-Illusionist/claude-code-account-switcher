use crate::config::AppConfig;
use crate::i18n::{I18n, Msg};
use crate::identity;

pub fn run(config: &AppConfig, i18n: &I18n) {
    let default_acc = config.get_default().ok().flatten();
    let accounts = config.list_accounts().unwrap_or_default();

    if accounts.is_empty() {
        i18n.print(Msg::ListEmpty);
        return;
    }

    i18n.print(Msg::ListHeader);
    for acc in &accounts {
        let acc_dir = config.account_path(acc);
        let info_suffix = identity_suffix(&acc_dir, i18n);
        if Some(acc.as_str()) == default_acc.as_deref() {
            println!("  ★ {}  {}{}", acc, i18n.msg(Msg::ListDefault), info_suffix);
        } else {
            println!("    {}{}", acc, info_suffix);
        }
    }
}

/// Returns "  email  3d ago" or "  email  3d ago *" if cached, else "".
/// `*` means current keychain token differs from the one cached at last
/// `doctor` run — usually a routine OAuth refresh, but could indicate a
/// silent re-auth. README has the full explanation.
fn identity_suffix(acc_dir: &std::path::Path, i18n: &I18n) -> String {
    let Some(cache) = identity::read_cache(acc_dir) else {
        return String::new();
    };
    let email = match cache.email {
        Some(e) => e,
        None => return String::new(),
    };
    let when = cache
        .fetched_at
        .and_then(identity::seconds_since)
        .map(|secs| i18n.msg(Msg::RelativeTime(secs)))
        .unwrap_or_default();

    let drift = match (
        cache.token_hash.as_deref(),
        identity::current_token_hash(acc_dir),
    ) {
        (Some(cached), Some(current)) if cached != current => " *",
        _ => "",
    };

    if when.is_empty() {
        format!("  {}{}", email, drift)
    } else {
        format!("  {}  {}{}", email, when, drift)
    }
}
