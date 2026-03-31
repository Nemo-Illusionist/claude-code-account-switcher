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
        let display = match &home {
            Some(h) => {
                let h_str = h.to_str().unwrap_or("");
                if dir.starts_with(h_str) {
                    format!("~{}", &dir[h_str.len()..])
                } else {
                    dir.clone()
                }
            }
            None => dir.clone(),
        };

        if Some(dir) == active_dir.as_ref() {
            println!("  {} → {}  {}", display, account, i18n.msg(Msg::LinksActive));
        } else {
            println!("  {} → {}", display, account);
        }
    }
}
