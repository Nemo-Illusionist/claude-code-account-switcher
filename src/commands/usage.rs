use crate::config::AppConfig;
use crate::i18n::{self, I18n, Msg};
use crate::identity::{self, Usage, UsageResult, UsageWindow};

const BAR_WIDTH: usize = 20;

pub fn run(config: &AppConfig, i18n: &I18n) {
    let default_acc = config.get_default().ok().flatten();
    let accounts = config.list_accounts().unwrap_or_default();

    // Same visibility rule as `list`: the standard ~/.claude/ row shows up only
    // if it has actually been logged into (live token or a prior doctor cache).
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

    i18n.print(Msg::UsageHeader);

    for acc in &accounts {
        let acc_dir = config.account_path(acc);
        let is_default = Some(acc.as_str()) == default_acc.as_deref();
        let marker = if is_default { "★" } else { " " };
        let suffix = email_suffix(identity::read_cache(&acc_dir).and_then(|c| c.email));
        println!("  {} {}{}", marker, acc, suffix);
        print_result(identity::fetch_account_usage(&acc_dir), i18n, acc);
    }

    if standard_logged_in {
        let cache = identity::read_cache_at(&identity::default_cache_path(&config.base_dir));
        let suffix = email_suffix(cache.and_then(|c| c.email));
        println!("    ~/.claude/{}  {}", suffix, i18n.msg(Msg::ListStandard));
        if let Some(dir) = standard.as_deref() {
            print_result(identity::fetch_account_usage(dir), i18n, "~/.claude/");
        }
    }
}

fn print_result(result: UsageResult, i18n: &I18n, name: &str) {
    match result {
        UsageResult::Ok(usage) => print_usage(&usage, i18n),
        UsageResult::NoToken => {
            println!("      {}", i18n.msg(Msg::DoctorNoToken(name.to_string())));
        }
        UsageResult::Offline => {
            println!("      {}", i18n.msg(Msg::DoctorOffline));
        }
    }
}

fn print_usage(usage: &Usage, i18n: &I18n) {
    if let Some(w) = &usage.five_hour {
        print_window("5h", w, i18n);
    }
    if let Some(w) = &usage.seven_day {
        print_window("7d", w, i18n);
    }
}

fn print_window(label: &str, w: &UsageWindow, i18n: &I18n) {
    let pct = w.utilization.clamp(0.0, 100.0);
    let reset = reset_label(w.resets_at.as_deref(), i18n);
    println!(
        "      {}  {}  {:>3}%  {}",
        label,
        bar(pct),
        pct.round() as i64,
        reset
    );
}

/// "resets in 2h 14m" / "available now" / "" (when the window has no reset).
fn reset_label(resets_at: Option<&str>, i18n: &I18n) -> String {
    let Some(resets_at) = resets_at else {
        return String::new();
    };
    match identity::seconds_until(resets_at) {
        Some(secs) if secs > 0 => i18n.msg(Msg::UsageResetsIn(i18n::forward_duration(
            secs as u64,
            i18n.lang,
        ))),
        Some(_) => i18n.msg(Msg::UsageAvailableNow),
        None => String::new(),
    }
}

fn email_suffix(email: Option<String>) -> String {
    match email {
        Some(e) => format!("  <{}>", e),
        None => String::new(),
    }
}

/// A 20-cell `[████░░░░…]` bar for a 0–100 percentage.
fn bar(pct: f64) -> String {
    let filled = ((pct / 100.0) * BAR_WIDTH as f64).round() as usize;
    let filled = filled.min(BAR_WIDTH);
    let mut s = String::with_capacity(BAR_WIDTH + 2);
    s.push('[');
    for _ in 0..filled {
        s.push('█');
    }
    for _ in 0..BAR_WIDTH - filled {
        s.push('░');
    }
    s.push(']');
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bar_empty_at_zero() {
        assert_eq!(bar(0.0), format!("[{}]", "░".repeat(BAR_WIDTH)));
    }

    #[test]
    fn bar_full_at_hundred() {
        assert_eq!(bar(100.0), format!("[{}]", "█".repeat(BAR_WIDTH)));
    }

    #[test]
    fn bar_half_at_fifty() {
        assert_eq!(bar(50.0), format!("[{}{}]", "█".repeat(10), "░".repeat(10)));
    }

    #[test]
    fn bar_clamps_overflow() {
        assert_eq!(bar(250.0), format!("[{}]", "█".repeat(BAR_WIDTH)));
    }

    #[test]
    fn email_suffix_formats_and_omits() {
        assert_eq!(email_suffix(Some("a@b.com".into())), "  <a@b.com>");
        assert_eq!(email_suffix(None), "");
    }
}
