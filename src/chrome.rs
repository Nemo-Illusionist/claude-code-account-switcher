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

    #[test]
    fn the_payload_is_the_one_claude_code_accepts() {
        // Its schema is `{enabledByDefault: boolean}` and nothing else: a
        // stray key makes it answer "Not a Claude in Chrome settings edit".
        // Written out by hand, not built from a serializer, so a change to
        // the shape has to be made deliberately. The trailing newline is
        // there because the subcommand reads a line from stdin.
        assert_eq!(payload(true), "{\"enabledByDefault\":true}\n");
        assert_eq!(payload(false), "{\"enabledByDefault\":false}\n");
    }
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

/// Turn Claude in Chrome on or off for a config dir, through Claude Code's
/// own subcommand.
///
/// We do not write the flag ourselves. It lives in `.claude.json`, the file
/// that also holds `oauthAccount` — the one thing this tool must never put a
/// wrong value into — so the write goes to `claude edit-chrome-settings
/// --json`, which is how the VS Code extension does it: a hidden
/// subcommand, `{"enabledByDefault": <bool>}` on stdin, reading
/// `CLAUDE_CONFIG_DIR` like everything else in Claude Code. Transcribed
/// from Claude Code 2.1.332; `claude --help` does not list it, so a future
/// build may rename it, which is why a failure is reported verbatim rather
/// than interpreted.
///
/// It refuses to run inside a Claude Code session — "This setting cannot be
/// changed from an editor started inside a Claude Code session" — which is
/// keyed on `CLAUDE_CODE_CHILD_SESSION`. That guard is Claude Code's, so it
/// is left alone and the message is passed through; the user runs the
/// command from a plain shell.
pub fn set_enabled(config_dir: &Path, on: bool) -> Result<(), SetError> {
    let payload = payload(on);
    let mut cmd = crate::windows_invocation::claude_command(&[
        "edit-chrome-settings".to_string(),
        "--json".to_string(),
    ])
    .map_err(|e| match e {
        // The only variant, and only reachable on Windows: a token cmd.exe
        // cannot carry. Ours are two fixed literals, so this cannot fire —
        // but it is not worth an `unwrap` to say so.
        crate::windows_invocation::InvocationError::UnsupportedArg(token) => {
            SetError::NotStarted(token)
        }
    })?;
    cmd.env("CLAUDE_CONFIG_DIR", config_dir);
    cmd.stdin(std::process::Stdio::piped());
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| SetError::NotStarted(e.to_string()))?;
    if let Some(stdin) = child.stdin.as_mut() {
        use std::io::Write;
        stdin
            .write_all(payload.as_bytes())
            .map_err(|e| SetError::NotStarted(e.to_string()))?;
    }
    let out = child
        .wait_with_output()
        .map_err(|e| SetError::NotStarted(e.to_string()))?;
    if out.status.success() {
        return Ok(());
    }
    // Whatever Claude Code said, said back. Both streams, because the
    // in-session refusal arrives on stdout while a usage error arrives on
    // stderr.
    let mut said = String::from_utf8_lossy(&out.stderr).trim().to_string();
    if said.is_empty() {
        said = String::from_utf8_lossy(&out.stdout).trim().to_string();
    }
    Err(SetError::Refused(said))
}

/// What `edit-chrome-settings` reads on stdin. Its schema is
/// `{enabledByDefault: boolean}` and nothing else — a stray key makes it
/// answer "Not a Claude in Chrome settings edit".
fn payload(on: bool) -> &'static str {
    if on {
        "{\"enabledByDefault\":true}\n"
    } else {
        "{\"enabledByDefault\":false}\n"
    }
}

#[derive(Debug)]
pub enum SetError {
    /// `claude` could not be run at all.
    NotStarted(String),
    /// It ran and said no. Carries what it said, verbatim.
    Refused(String),
}
