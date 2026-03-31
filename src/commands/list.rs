use crate::config::AppConfig;
use crate::i18n::{I18n, Msg};

pub fn run(config: &AppConfig, i18n: &I18n) {
    let default_acc = config.get_default().ok().flatten();
    let accounts = config.list_accounts().unwrap_or_default();

    if accounts.is_empty() {
        i18n.print(Msg::ListEmpty);
        return;
    }

    i18n.print(Msg::ListHeader);
    for acc in &accounts {
        if Some(acc.as_str()) == default_acc.as_deref() {
            println!("  ★ {}  {}", acc, i18n.msg(Msg::ListDefault));
        } else {
            println!("    {}", acc);
        }
    }
}
