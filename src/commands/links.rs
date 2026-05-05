use crate::config::AppConfig;
use crate::i18n::{I18n, Msg};
use crate::resolve;

pub fn run(config: &AppConfig, i18n: &I18n) {
    let links = config.all_links().unwrap_or_default();
    if links.is_empty() {
        i18n.print(Msg::LinksEmpty);
        return;
    }

    i18n.print(Msg::LinksHeader);

    let cwd = std::env::current_dir().ok();
    let active_dir = cwd
        .as_deref()
        .and_then(|d| resolve::find_linked_dir(config, d));

    let home = dirs::home_dir();
    let mut sorted = links;
    sorted.sort_by(|a, b| a.0.cmp(&b.0));

    for (dir, account) in &sorted {
        let display = home
            .as_deref()
            .and_then(|h| h.to_str())
            .and_then(|h_str| dir.strip_prefix(h_str).map(|rest| format!("~{}", rest)))
            .unwrap_or_else(|| dir.clone());

        if Some(dir) == active_dir.as_ref() {
            println!(
                "  {} → {}  {}",
                display,
                account,
                i18n.msg(Msg::LinksActive)
            );
        } else {
            println!("  {} → {}", display, account);
        }
    }
}
