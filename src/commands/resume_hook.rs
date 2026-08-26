use crate::config::AppConfig;
use crate::i18n::{I18n, Msg};
use crate::sessions;

/// Show or set whether the generated `claude` wrapper checks a `--resume <id>`
/// against the other accounts before handing over. No argument shows the
/// current state.
pub fn run(config: &AppConfig, i18n: &I18n, state: Option<&str>) -> i32 {
    let Some(state) = state else {
        print_state(config, i18n);
        return 0;
    };

    let Some(on) = parse_state(state) else {
        i18n.print(Msg::ResumeHookInvalid(state.to_string()));
        return 1;
    };

    let value = if on { "on" } else { "off" };
    if let Err(e) = config.set_setting(super::session::RESUME_HOOK_SETTING, value) {
        i18n.print(Msg::ResumeHookWriteFailed(e.to_string()));
        return 1;
    }
    i18n.print(Msg::ResumeHookSet(value.to_string()));
    if on {
        i18n.print(Msg::ResumeHookExplainOn);
    } else {
        i18n.print(Msg::ResumeHookExplainOff);
    }
    0
}

fn print_state(config: &AppConfig, i18n: &I18n) {
    let stored = config
        .get_setting(super::session::RESUME_HOOK_SETTING)
        .unwrap_or_else(|| "on".to_string());
    i18n.print(Msg::ResumeHookState(stored));
    // The env override wins over the stored value, so say so rather than
    // letting the two disagree silently.
    if !super::session::hook_enabled(config) && !is_off(config) {
        i18n.print(Msg::ResumeHookEnvOverride);
    }
    if !sessions::account_config_dirs(config).is_empty() {
        i18n.print(Msg::ResumeHookHint);
    }
}

fn is_off(config: &AppConfig) -> bool {
    config
        .get_setting(super::session::RESUME_HOOK_SETTING)
        .as_deref()
        == Some("off")
}

/// Accept the spellings people actually type. Returns `None` for anything
/// else so a typo turns into an error rather than a silent "off".
fn parse_state(v: &str) -> Option<bool> {
    match v.trim().to_lowercase().as_str() {
        "on" | "true" | "yes" | "1" | "enable" | "enabled" => Some(true),
        "off" | "false" | "no" | "0" | "disable" | "disabled" => Some(false),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_obvious_spellings() {
        for v in ["on", "ON", " on ", "true", "yes", "1", "enable", "enabled"] {
            assert_eq!(parse_state(v), Some(true), "{v}");
        }
        for v in ["off", "OFF", "false", "no", "0", "disable", "disabled"] {
            assert_eq!(parse_state(v), Some(false), "{v}");
        }
    }

    #[test]
    fn rejects_anything_else_rather_than_guessing() {
        // A typo silently meaning "off" would disable the check without
        // anyone noticing.
        assert_eq!(parse_state("of"), None);
        assert_eq!(parse_state("nope"), None);
        assert_eq!(parse_state(""), None);
    }
}
