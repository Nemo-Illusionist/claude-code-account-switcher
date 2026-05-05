use std::path::Path;

use crate::config::AppConfig;
use crate::i18n::{I18n, Msg};
use crate::identity::{self, CachedInfo};

pub fn run(config: &AppConfig, i18n: &I18n) {
    let default_acc = config.get_default().ok().flatten();
    let accounts = config.list_accounts().unwrap_or_default();

    // Standard ~/.claude/ shows up only if the user has actually logged into
    // the standard config — so empty installs don't get a phantom row.
    let standard = identity::standard_token_dir();
    let standard_logged_in = standard
        .as_deref()
        .and_then(identity::current_token_hash)
        .is_some()
        || identity::read_cache_at(&identity::default_cache_path(&config.base_dir)).is_some();

    if accounts.is_empty() && !standard_logged_in {
        i18n.print(Msg::ListEmpty);
        return;
    }

    i18n.print(Msg::ListHeader);

    for acc in &accounts {
        let acc_dir = config.account_path(acc);
        let cache = identity::read_cache(&acc_dir);
        let info_suffix = cache_suffix(
            cache.as_ref(),
            Some(acc_dir.as_path()),
            i18n,
            /* skip_label */ "",
        );
        if Some(acc.as_str()) == default_acc.as_deref() {
            println!("  ★ {}  {}{}", acc, i18n.msg(Msg::ListDefault), info_suffix);
        } else {
            println!("    {}{}", acc, info_suffix);
        }
    }

    if standard_logged_in {
        let cache_path = identity::default_cache_path(&config.base_dir);
        let cache = identity::read_cache_at(&cache_path);
        let suffix = cache_suffix(
            cache.as_ref(),
            standard.as_deref(),
            i18n,
            &format!("  {}", i18n.msg(Msg::ListStandard)),
        );
        println!("    ~/.claude/{}", suffix);
    }
}

/// Returns "  email  3d ago [skip_label]" or "  email  3d ago * [skip_label]"
/// if cache is present, else just `skip_label` if any.
///
/// `*` means current token at `token_dir` differs from the one cached at
/// last `doctor` run (see README — usually a routine OAuth refresh).
/// `skip_label` is appended after the time/marker, used by the standard row
/// to add `(standard)`.
fn cache_suffix(
    cache: Option<&CachedInfo>,
    token_dir: Option<&Path>,
    i18n: &I18n,
    skip_label: &str,
) -> String {
    let Some(cache) = cache else {
        return skip_label.to_string();
    };
    let Some(email) = cache.email.as_deref() else {
        return skip_label.to_string();
    };
    let when = cache
        .fetched_at
        .and_then(identity::seconds_since)
        .map(|secs| i18n.msg(Msg::RelativeTime(secs)))
        .unwrap_or_default();

    let drift = match (
        cache.token_hash.as_deref(),
        token_dir.and_then(identity::current_token_hash),
    ) {
        (Some(cached), Some(current)) if cached != current => " *",
        _ => "",
    };

    let body = if when.is_empty() {
        format!("  {}{}", email, drift)
    } else {
        format!("  {}  {}{}", email, when, drift)
    };
    format!("{}{}", body, skip_label)
}
