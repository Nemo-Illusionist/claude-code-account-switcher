use crate::config::AppConfig;
use crate::i18n::{I18n, Msg};
use crate::identity;
use crate::resolve;

pub fn run(config: &AppConfig, i18n: &I18n) {
    let cwd = std::env::current_dir().expect("Cannot get current directory");

    let linked_dir = resolve::find_linked_dir(config, &cwd);

    if let Some(ref ld) = linked_dir {
        let account = config.get_link(ld).ok().flatten();
        if let Some(ref acc) = account {
            let dir_name = std::path::Path::new(ld)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(ld);
            let info = i18n.msg(Msg::StatusLinked(dir_name.to_string()));
            i18n.print(Msg::StatusActive(label(config, acc), info));
            return;
        }
    }

    if let Ok(Some(ref acc)) = config.get_default() {
        let info = i18n.msg(Msg::StatusDefault);
        i18n.print(Msg::StatusActive(label(config, acc), info));
        return;
    }

    // Standard ~/.claude/ — show email if doctor cached one for it.
    let standard_label = standard_label(config);
    if standard_label != "~/.claude/" {
        // Cache exists; format follows StatusActive for visual consistency.
        i18n.print(Msg::StatusActive(
            standard_label,
            i18n.msg(Msg::ListStandard),
        ));
    } else {
        i18n.print(Msg::StatusStandard);
    }
}

/// "<acc>" or "<acc> <email>" or "<acc> <email *>" depending on what's
/// cached. The trailing `*` flags drift between cached and current
/// keychain token (see commands/list.rs for the full convention).
fn label(config: &AppConfig, acc: &str) -> String {
    let acc_dir = config.account_path(acc);
    let Some(cache) = identity::read_cache(&acc_dir) else {
        return acc.to_string();
    };
    let Some(email) = cache.email else {
        return acc.to_string();
    };
    let drift = match (
        cache.token_hash.as_deref(),
        identity::current_token_hash(&acc_dir),
    ) {
        (Some(cached), Some(current)) if cached != current => " *",
        _ => "",
    };
    format!("{} <{}{}>", acc, email, drift)
}

/// "~/.claude/" or "~/.claude/ <email>" or "~/.claude/ <email *>".
fn standard_label(config: &AppConfig) -> String {
    let cache_path = identity::default_cache_path(&config.base_dir);
    let Some(cache) = identity::read_cache_at(&cache_path) else {
        return "~/.claude/".to_string();
    };
    let Some(email) = cache.email else {
        return "~/.claude/".to_string();
    };
    let drift = match (
        cache.token_hash.as_deref(),
        identity::standard_token_dir()
            .as_deref()
            .and_then(identity::current_token_hash),
    ) {
        (Some(cached), Some(current)) if cached != current => " *",
        _ => "",
    };
    format!("~/.claude/ <{}{}>", email, drift)
}
