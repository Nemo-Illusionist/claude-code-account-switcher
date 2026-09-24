//! Whether Claude Code's "Claude in Chrome" is switched on for a config dir.
//!
//! Claude Code wires the `claude-in-chrome` MCP server per config dir, from
//! `claudeInChromeDefaultEnabled` in that dir's own `.claude.json`. Nothing
//! carries the flag into a new account — and a value that is present but not
//! `true` also takes the account off the auto-enable path, which only ever
//! fires when the key is absent entirely. So an account can sit permanently
//! without the browser tools, with nothing on screen to say why: they are
//! simply not among the tools, which reads as "this machine doesn't have the
//! extension" rather than "this account never said yes".
//!
//! We only read. Turning it on writes to `.claude.json` — the file that also
//! holds `oauthAccount`, and the one thing this tool must never put a wrong
//! value into — so that stays with Claude Code's own `/chrome`.

use std::path::Path;

use crate::identity::local_identity_path;

fn config_json(config_dir: &Path) -> Option<serde_json::Value> {
    let path = local_identity_path(config_dir)?;
    let raw = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

/// Whether Claude in Chrome is switched on for this config dir.
///
/// `None` means we have no answer — no `.claude.json` yet, or one that does
/// not parse. A config dir Claude Code has never written is a brand-new
/// account, not an account that said no, so it is never reported as off.
pub fn enabled(config_dir: &Path) -> Option<bool> {
    let v = config_json(config_dir)?;
    Some(
        v.get("claudeInChromeDefaultEnabled")
            .and_then(|x| x.as_bool())
            == Some(true),
    )
}

/// Whether this config dir has ever met the Chrome extension.
///
/// Claude Code caches that it found one installed, and records a device once
/// one pairs. Either is evidence the feature is actually in use here, which is
/// what separates "off because you don't use this" — the correct state, and
/// none of our business — from "off although you do".
pub fn extension_seen(config_dir: &Path) -> bool {
    let Some(v) = config_json(config_dir) else {
        return false;
    };
    if v.get("cachedChromeExtensionInstalled")
        .and_then(|x| x.as_bool())
        == Some(true)
    {
        return true;
    }
    v.get("chromeExtension")
        .and_then(|x| x.get("pairedDeviceId"))
        .and_then(|x| x.as_str())
        .is_some_and(|s| !s.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("claude-acc-chrome-{}-{}", name, std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn with_config(name: &str, body: &str) -> PathBuf {
        let dir = scratch(name);
        fs::write(dir.join(".claude.json"), body).unwrap();
        dir
    }

    #[test]
    fn true_is_on() {
        let dir = with_config("on", r#"{"claudeInChromeDefaultEnabled": true}"#);
        assert_eq!(enabled(&dir), Some(true));
    }

    #[test]
    fn false_is_off() {
        let dir = with_config("off", r#"{"claudeInChromeDefaultEnabled": false}"#);
        assert_eq!(enabled(&dir), Some(false));
    }

    // The case this whole module exists for. Claude Code leaves the key at
    // `null` rather than removing it, and `null` is not `true`, so the server
    // is never wired — while also not being absent, so the auto-enable path
    // that would have offered to turn it on never fires either.
    #[test]
    fn null_is_off_not_unknown() {
        let dir = with_config("null", r#"{"claudeInChromeDefaultEnabled": null}"#);
        assert_eq!(enabled(&dir), Some(false));
    }

    #[test]
    fn absent_key_is_off() {
        let dir = with_config("absent", r#"{"oauthAccount": {}}"#);
        assert_eq!(enabled(&dir), Some(false));
    }

    // No file and unparseable file are both "we don't know", never "off" —
    // reporting a fresh account dir as switched off would be a finding about
    // nothing.
    #[test]
    fn missing_file_has_no_answer() {
        let dir = scratch("missing");
        assert_eq!(enabled(&dir), None);
    }

    #[test]
    fn unparseable_file_has_no_answer() {
        let dir = with_config("garbage", "{not json");
        assert_eq!(enabled(&dir), None);
    }

    #[test]
    fn cached_install_counts_as_seen() {
        let dir = with_config("cached", r#"{"cachedChromeExtensionInstalled": true}"#);
        assert!(extension_seen(&dir));
    }

    #[test]
    fn paired_device_counts_as_seen() {
        let dir = with_config(
            "paired",
            r#"{"chromeExtension": {"pairedDeviceId": "abc"}}"#,
        );
        assert!(extension_seen(&dir));
    }

    // `chromeExtension: null` is what Claude Code writes when nothing has
    // paired — it must not read as a pairing.
    #[test]
    fn null_extension_is_not_seen() {
        let dir = with_config(
            "unpaired",
            r#"{"chromeExtension": null, "cachedChromeExtensionInstalled": false}"#,
        );
        assert!(!extension_seen(&dir));
    }

    #[test]
    fn nothing_recorded_is_not_seen() {
        let dir = with_config("bare", r#"{"oauthAccount": {}}"#);
        assert!(!extension_seen(&dir));
    }
}
