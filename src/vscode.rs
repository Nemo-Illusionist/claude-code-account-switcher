// VS Code integration: `claudeCode.claudeProcessWrapper`.
//
// Problem: the extension's *native UI* does not resolve `claude` from
// PATH. It runs the binary it ships (`resources/native-binary/claude`)
// with `{...process.env}` plus the machine-scoped
// `claudeCode.environmentVariables`. So the `~/.claude-switch/bin/claude`
// wrapper never runs, and `CLAUDE_CONFIG_DIR` can only be whatever the
// extension host inherited at startup — one value for every workspace.
// (Terminal mode, `claudeCode.useTerminal`, does go through PATH and has
// always worked.)
//
// Fix: `claudeCode.claudeProcessWrapper` is an executable the extension
// calls *instead of* the bundled binary, passing that binary as the first
// argument and running it with cwd set to the workspace folder. Point it
// at a wrapper of ours that derives `CLAUDE_CONFIG_DIR` from the cwd and
// execs the real thing, and the native UI picks the account per workspace.
//
// The settings file is JSONC — comments and trailing commas are legal and
// people have them. So it is edited as text, replacing exactly the one
// key's value, rather than parsed and re-serialised: round-tripping it
// through serde would silently delete every comment in someone's editor
// config.

use std::fs;
use std::path::{Path, PathBuf};

/// The setting the extension reads to find a launcher to call instead of
/// its bundled binary.
pub const WRAPPER_SETTING: &str = "claudeCode.claudeProcessWrapper";

/// Filename of our launcher inside `~/.claude-switch/bin/`. Deliberately
/// not `claude`: that name is the PATH wrapper, which has a different
/// calling convention.
pub const WRAPPER_NAME: &str = "claude-vscode";

#[cfg(not(windows))]
const VSCODE_WRAPPER_TEMPLATE: &str = include_str!("../shell/claude-vscode-wrapper.sh");
#[cfg(not(windows))]
const WRAPPER_PLACEHOLDER: &str = "__CLAUDE_ACC_BIN__";

/// A VS Code-family editor installed on this machine.
pub struct Editor {
    pub label: &'static str,
    pub settings: PathBuf,
}

/// Editors that ship the Claude Code extension and read the same setting.
/// The second field is the directory name each uses under the platform's
/// config root — identical layout on all three platforms, which is why
/// `dirs::config_dir()` covers macOS (`~/Library/Application Support`),
/// Linux (`~/.config`) and Windows (`%APPDATA%`) with one path.
const KNOWN_EDITORS: &[(&str, &str)] = &[
    ("VS Code", "Code"),
    ("VS Code Insiders", "Code - Insiders"),
    ("VSCodium", "VSCodium"),
    ("Cursor", "Cursor"),
];

/// `<config root>/<dir>/User/settings.json`, wherever that is here.
pub fn settings_path(dir: &str) -> Option<PathBuf> {
    dirs::config_dir().map(|c| c.join(dir).join("User").join("settings.json"))
}

/// Editors actually installed: the `User` directory exists. `settings.json`
/// itself may not — a profile that has never been customised has none, and
/// writing one is fine.
pub fn detect_editors() -> Vec<Editor> {
    KNOWN_EDITORS
        .iter()
        .filter_map(|(label, dir)| {
            let settings = settings_path(dir)?;
            settings.parent()?.is_dir().then_some(Editor {
                label,
                settings: settings.clone(),
            })
        })
        .collect()
}

/// Where the wrapper lives once installed.
pub fn wrapper_path(base_dir: &Path) -> PathBuf {
    base_dir.join("bin").join(WRAPPER_NAME)
}

/// Write `~/.claude-switch/bin/claude-vscode`. Always overwrites, like the
/// PATH wrapper does — a stale one pointing at a moved `claude-acc` is
/// worse than the write.
#[cfg(not(windows))]
pub fn install_wrapper(base_dir: &Path, claude_acc_bin: &Path) -> std::io::Result<PathBuf> {
    use std::os::unix::fs::PermissionsExt;
    let bin_dir = base_dir.join("bin");
    fs::create_dir_all(&bin_dir)?;
    let wrapper = bin_dir.join(WRAPPER_NAME);
    let content =
        VSCODE_WRAPPER_TEMPLATE.replace(WRAPPER_PLACEHOLDER, &claude_acc_bin.to_string_lossy());
    fs::write(&wrapper, content)?;
    fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o755))?;
    Ok(wrapper)
}

/// What `settings.json` currently says about the wrapper setting.
pub enum WrapperState {
    /// The key is absent.
    Unset,
    /// The key points at our wrapper.
    Ours,
    /// The key points at something else — another tool, or a hand-written
    /// path. Never overwritten without `--force`.
    Foreign(String),
    /// The file is there but we can't safely edit it: it isn't a JSON
    /// object, isn't valid UTF-8, or couldn't be read at all.
    Unreadable,
}

/// Read a settings file, distinguishing "there is no file" from "there is a
/// file we could not read".
///
/// `read_to_string` collapses the two, and the difference decides whether the
/// next step creates a fresh object or refuses to touch anything. A file
/// holding one non-UTF-8 byte, or one we lack permission to read, must never
/// look like an absent file.
fn read_settings(settings: &Path) -> Result<Option<String>, std::io::Error> {
    match fs::read_to_string(settings) {
        Ok(s) => Ok(Some(s)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

/// Read the state of `WRAPPER_SETTING` in a settings file that may not exist.
pub fn wrapper_state(settings: &Path, ours: &Path) -> WrapperState {
    let src = match read_settings(settings) {
        Ok(Some(s)) => s,
        Ok(None) => return WrapperState::Unset,
        Err(_) => return WrapperState::Unreadable,
    };
    if src.trim().is_empty() {
        return WrapperState::Unset;
    }
    match find_top_level_key(&src, WRAPPER_SETTING) {
        Scan::Malformed => WrapperState::Unreadable,
        Scan::Absent => WrapperState::Unset,
        Scan::Found(span) => {
            match serde_json::from_str::<String>(&src[span.value_start..span.value_end]) {
                Ok(v) if Path::new(&v) == ours => WrapperState::Ours,
                Ok(v) => WrapperState::Foreign(v),
                // The key is there holding something that isn't a string.
                // Not ours, and not something to overwrite blind.
                Err(_) => {
                    WrapperState::Foreign(src[span.value_start..span.value_end].trim().to_string())
                }
            }
        }
    }
}

/// Point the setting at `wrapper`, creating the file if needed. Returns
/// `false` when the existing file isn't something we can edit without
/// risking it — the caller reports that rather than clobbering it.
pub fn set_wrapper(settings: &Path, wrapper: &Path) -> std::io::Result<bool> {
    // Regression: this used to be `read_to_string(...).unwrap_or_default()`,
    // so a settings.json that failed to read — one stray non-UTF-8 byte was
    // enough — looked empty and was replaced wholesale with an object holding
    // only our key. Someone's entire editor config, gone, reported as success.
    let src = read_settings(settings)?.unwrap_or_default();
    let Some(out) = set_string_value(&src, WRAPPER_SETTING, &wrapper.to_string_lossy()) else {
        return Ok(false);
    };
    if let Some(parent) = settings.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(settings, out)?;
    Ok(true)
}

/// Drop the setting. Returns `false` when there was nothing to remove.
pub fn clear_wrapper(settings: &Path) -> std::io::Result<bool> {
    let Some(src) = read_settings(settings)? else {
        return Ok(false);
    };
    let Some(out) = remove_key(&src, WRAPPER_SETTING) else {
        return Ok(false);
    };
    fs::write(settings, out)?;
    Ok(true)
}

// ---------------------------------------------------------------------
// JSONC text editing
//
// Value-in, value-out on purpose: every decision below is testable without
// a filesystem, and the functions above are the only part that touches one.
// ---------------------------------------------------------------------

/// Byte spans of one top-level `"key": value` entry.
struct KeySpan {
    /// Opening quote of the key.
    key_start: usize,
    /// First byte of the value.
    value_start: usize,
    /// One past the last byte of the value.
    value_end: usize,
    /// One past the entry's trailing comma, if it has one; `value_end`
    /// otherwise.
    entry_end: usize,
    /// Offset of the comma separating the previous entry from this one, if
    /// there is a previous entry. Recorded during the scan, which already
    /// knows what is a comment and what is not — deleting the last entry has
    /// to take that comma with it, and finding it by searching backwards
    /// through raw text finds commas inside comments instead.
    prev_comma: Option<usize>,
}

/// Outcome of looking for one top-level key.
enum Scan {
    Found(KeySpan),
    /// Scanned to the end of a well-formed object; the key isn't in it.
    Absent,
    /// The text is not an object we can scan — truncated, unterminated
    /// string, something that isn't an object at all. Nothing may be
    /// written to it.
    Malformed,
}

/// The string value of a top-level key, if it is a string. Only the tests
/// need this on its own; `wrapper_state` reads the span directly so it can
/// tell a non-string value apart from an absent key.
#[cfg(test)]
pub fn read_string_value(src: &str, key: &str) -> Option<String> {
    let Scan::Found(span) = find_top_level_key(src, key) else {
        return None;
    };
    serde_json::from_str::<String>(&src[span.value_start..span.value_end]).ok()
}

/// `src` with the top-level `key` set to the string `value`, everything
/// else — comments, ordering, indentation — left as it was. `None` when
/// the text isn't an object we can edit safely.
pub fn set_string_value(src: &str, key: &str, value: &str) -> Option<String> {
    let encoded = serde_json::to_string(value).ok()?;

    if src.trim().is_empty() {
        return Some(format!("{{\n    \"{}\": {}\n}}\n", key, encoded));
    }
    // Regression: a truncated file — `{\n    "` is enough — used to reach
    // the insert path below and get our key spliced into something that was
    // never a complete object. Only a clean scan may be written to.
    match find_top_level_key(src, key) {
        Scan::Malformed => return None,
        Scan::Found(span) => {
            let mut out = String::with_capacity(src.len() + encoded.len());
            out.push_str(&src[..span.value_start]);
            out.push_str(&encoded);
            out.push_str(&src[span.value_end..]);
            return Some(out);
        }
        Scan::Absent => {}
    }

    let b = src.as_bytes();
    let brace = skip_ws_and_comments(b, 0);
    let indent = detect_indent(src);
    let has_entries = first_key_start(src).is_some();
    // With entries, the new one goes first and needs a comma after it.
    // Without, it is the only one and must not have one.
    let insert = if has_entries {
        format!("\n{}\"{}\": {},", indent, key, encoded)
    } else {
        format!("\n{}\"{}\": {}\n", indent, key, encoded)
    };

    let mut out = String::with_capacity(src.len() + insert.len());
    out.push_str(&src[..brace + 1]);
    out.push_str(&insert);
    out.push_str(&src[brace + 1..]);
    Some(out)
}

/// `src` with the top-level `key` and its value gone. `None` when the key
/// wasn't there.
pub fn remove_key(src: &str, key: &str) -> Option<String> {
    let Scan::Found(span) = find_top_level_key(src, key) else {
        return None;
    };

    // Take the whole line when nothing but whitespace precedes the key on
    // it — otherwise removing the entry leaves a stranded blank line.
    let line_start = src[..span.key_start]
        .rfind('\n')
        .map(|p| p + 1)
        .unwrap_or(0);
    let own_line = src[line_start..span.key_start].trim().is_empty();
    let start = if own_line { line_start } else { span.key_start };
    let mut end = span.entry_end;

    if own_line
        && let Some(nl) = src[end..].find('\n')
        && src[end..end + nl].trim().is_empty()
    {
        end += nl + 1;
    }

    // Last entry with no comma of its own: the comma that separated it from
    // the previous entry is now trailing. Legal in JSONC, but leave the file
    // tidy.
    //
    // Regression: this used to search backwards through the raw text for a
    // comma, which happily found one inside a preceding line comment
    // (`// dark, light, high contrast,`) and cut from the middle of the
    // comment to the end of our entry — taking the closing brace with it and
    // leaving a file VS Code rejects, resetting every setting to its default.
    // The scan records where the real comma is instead.
    //
    // It is excised on its own rather than by widening the range back to it:
    // anything between that comma and our entry — a trailing comment on the
    // previous line, a standalone comment block — belongs to the file, not to
    // us, and must survive.
    let comma = (span.entry_end == span.value_end)
        .then_some(span.prev_comma)
        .flatten()
        .filter(|c| *c < start);

    let mut out = String::with_capacity(src.len());
    match comma {
        Some(c) => {
            out.push_str(&src[..c]);
            out.push_str(&src[c + 1..start]);
        }
        None => out.push_str(&src[..start]),
    }
    out.push_str(&src[end..]);
    Some(out)
}

/// Indentation of the first top-level entry, so an inserted one matches.
/// Four spaces is VS Code's own default and the fallback.
fn detect_indent(src: &str) -> String {
    let Some(key_start) = first_key_start(src) else {
        return "    ".to_string();
    };
    let line_start = src[..key_start].rfind('\n').map(|p| p + 1).unwrap_or(0);
    let prefix = &src[line_start..key_start];
    if prefix.trim().is_empty() && !prefix.is_empty() {
        prefix.to_string()
    } else {
        "    ".to_string()
    }
}

/// Offset of the first top-level key's opening quote, if the object has one.
fn first_key_start(src: &str) -> Option<usize> {
    let b = src.as_bytes();
    let mut i = skip_ws_and_comments(b, 0);
    if i >= b.len() || b[i] != b'{' {
        return None;
    }
    i = skip_ws_and_comments(b, i + 1);
    (i < b.len() && b[i] == b'"').then_some(i)
}

fn find_top_level_key(src: &str, key: &str) -> Scan {
    let b = src.as_bytes();
    let mut i = skip_ws_and_comments(b, 0);
    if i >= b.len() || b[i] != b'{' {
        return Scan::Malformed;
    }
    i += 1;
    let mut prev_comma = None;

    loop {
        i = skip_ws_and_comments(b, i);
        // A well-formed object ends at its closing brace. Running off the
        // end instead means the file is truncated.
        if i >= b.len() {
            return Scan::Malformed;
        }
        if b[i] == b'}' {
            return Scan::Absent;
        }
        if b[i] == b',' {
            prev_comma = Some(i);
            i += 1;
            continue;
        }
        if b[i] != b'"' {
            // Not something we understand — refuse rather than guess.
            return Scan::Malformed;
        }

        let key_start = i;
        // Regression: an unterminated key string used to come back as
        // `b.len()`, and `key_end - 1` then either ran backwards past
        // `key_start + 1` or landed inside a multi-byte character — slicing
        // `src` with it panicked, taking `claude-acc install` down with it
        // (it calls this through the VS Code hint).
        let Some(key_end) = skip_string(b, i) else {
            return Scan::Malformed;
        };
        let name = &src[key_start + 1..key_end - 1];

        let colon = skip_ws_and_comments(b, key_end);
        if colon >= b.len() || b[colon] != b':' {
            return Scan::Malformed;
        }
        let value_start = skip_ws_and_comments(b, colon + 1);
        let value_end = skip_value(b, value_start);

        if name == key {
            let after = skip_ws_and_comments(b, value_end);
            let entry_end = if after < b.len() && b[after] == b',' {
                after + 1
            } else {
                value_end
            };
            return Scan::Found(KeySpan {
                key_start,
                value_start,
                value_end,
                entry_end,
                prev_comma,
            });
        }
        i = value_end;
    }
}

fn skip_ws_and_comments(b: &[u8], mut i: usize) -> usize {
    loop {
        while i < b.len() && b[i].is_ascii_whitespace() {
            i += 1;
        }
        if i + 1 < b.len() && b[i] == b'/' && b[i + 1] == b'/' {
            while i < b.len() && b[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if i + 1 < b.len() && b[i] == b'/' && b[i + 1] == b'*' {
            i += 2;
            while i + 1 < b.len() && !(b[i] == b'*' && b[i + 1] == b'/') {
                i += 1;
            }
            i = (i + 2).min(b.len());
            continue;
        }
        return i;
    }
}

/// One past the closing quote of the string starting at `i`, or `None` when
/// the string is never closed. Callers that slice on the result must treat
/// `None` as "this text is not editable" — a made-up offset here is how a
/// truncated file turns into a panic or a mangled write.
fn skip_string(b: &[u8], mut i: usize) -> Option<usize> {
    i += 1;
    while i < b.len() {
        match b[i] {
            b'\\' => i += 2,
            b'"' => return Some(i + 1),
            _ => i += 1,
        }
    }
    None
}

/// One past the last byte of the value starting at `i`.
fn skip_value(b: &[u8], i: usize) -> usize {
    let mut i = skip_ws_and_comments(b, i);
    if i >= b.len() {
        return i;
    }
    match b[i] {
        b'"' => skip_string(b, i).unwrap_or(b.len()),
        open @ (b'{' | b'[') => {
            let close = if open == b'{' { b'}' } else { b']' };
            let mut depth = 0usize;
            while i < b.len() {
                match b[i] {
                    b'"' => i = skip_string(b, i).unwrap_or(b.len()),
                    b'/' if i + 1 < b.len() && (b[i + 1] == b'/' || b[i + 1] == b'*') => {
                        i = skip_ws_and_comments(b, i)
                    }
                    c if c == open => {
                        depth += 1;
                        i += 1;
                    }
                    c if c == close => {
                        depth -= 1;
                        i += 1;
                        if depth == 0 {
                            return i;
                        }
                    }
                    _ => i += 1,
                }
            }
            i
        }
        // number / true / false / null
        _ => {
            while i < b.len()
                && !matches!(b[i], b',' | b'}' | b']' | b'/')
                && !b[i].is_ascii_whitespace()
            {
                i += 1;
            }
            i
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &str = WRAPPER_SETTING;

    #[test]
    fn reads_the_value_of_an_existing_key() {
        let src = r#"{ "editor.fontSize": 13, "claudeCode.claudeProcessWrapper": "/tmp/w" }"#;
        assert_eq!(read_string_value(src, KEY), Some("/tmp/w".to_string()));
    }

    #[test]
    fn reads_nothing_when_the_key_is_absent() {
        assert_eq!(read_string_value(r#"{ "editor.fontSize": 13 }"#, KEY), None);
    }

    #[test]
    fn a_nested_key_of_the_same_name_is_not_the_top_level_one() {
        // Only top-level settings are real settings; one nested inside
        // another object's value must not be mistaken for ours.
        let src = r#"{ "some.tool": { "claudeCode.claudeProcessWrapper": "/nested" } }"#;
        assert_eq!(read_string_value(src, KEY), None);
    }

    #[test]
    fn setting_an_existing_key_replaces_only_its_value() {
        let src = "{\n    // keep me\n    \"editor.fontSize\": 13,\n    \"claudeCode.claudeProcessWrapper\": \"/old\"\n}\n";
        let out = set_string_value(src, KEY, "/new").unwrap();
        assert!(out.contains("// keep me"), "comment was dropped: {out}");
        assert!(out.contains("\"editor.fontSize\": 13"));
        assert_eq!(read_string_value(&out, KEY), Some("/new".to_string()));
    }

    #[test]
    fn comments_and_trailing_commas_survive_an_insert() {
        // Regression: settings.json is JSONC. Parsing it with serde and
        // writing it back would delete every comment in the file.
        let src = "{\n    // fonts\n    \"editor.fontSize\": 13,\n    /* block */\n    \"files.autoSave\": \"off\",\n}\n";
        let out = set_string_value(src, KEY, "/w").unwrap();
        assert!(out.contains("// fonts"));
        assert!(out.contains("/* block */"));
        assert!(out.contains("\"files.autoSave\": \"off\","));
        assert_eq!(read_string_value(&out, KEY), Some("/w".to_string()));
    }

    #[test]
    fn a_key_commented_out_is_not_treated_as_present() {
        let src = "{\n    // \"claudeCode.claudeProcessWrapper\": \"/old\",\n    \"editor.fontSize\": 13\n}\n";
        assert_eq!(read_string_value(src, KEY), None);
        let out = set_string_value(src, KEY, "/new").unwrap();
        assert_eq!(read_string_value(&out, KEY), Some("/new".to_string()));
        assert!(out.contains("// \"claudeCode.claudeProcessWrapper\": \"/old\","));
    }

    #[test]
    fn inserting_into_a_populated_object_keeps_it_valid() {
        let src = "{\n    \"editor.fontSize\": 13\n}\n";
        let out = set_string_value(src, KEY, "/w").unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(parsed[KEY], "/w");
        assert_eq!(parsed["editor.fontSize"], 13);
    }

    #[test]
    fn inserting_into_an_empty_object_keeps_it_valid() {
        let out = set_string_value("{}\n", KEY, "/w").unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(parsed[KEY], "/w");
    }

    #[test]
    fn an_empty_or_missing_file_becomes_a_new_object() {
        for src in ["", "   \n\n"] {
            let out = set_string_value(src, KEY, "/w").unwrap();
            let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
            assert_eq!(parsed[KEY], "/w");
        }
    }

    #[test]
    fn a_file_that_is_not_an_object_is_refused_rather_than_clobbered() {
        // Someone's settings.json holding an array, or a half-typed file:
        // better to report it than to replace it with our own object.
        assert!(set_string_value("[1, 2]", KEY, "/w").is_none());
        assert!(set_string_value("not json at all", KEY, "/w").is_none());
    }

    #[test]
    fn the_inserted_entry_matches_the_files_indentation() {
        let src = "{\n  \"editor.fontSize\": 13\n}\n";
        let out = set_string_value(src, KEY, "/w").unwrap();
        assert!(
            out.contains("\n  \"claudeCode.claudeProcessWrapper\""),
            "expected two-space indent: {out}"
        );
    }

    #[test]
    fn a_tab_indented_file_gets_a_tab_indented_entry() {
        let src = "{\n\t\"editor.fontSize\": 13\n}\n";
        let out = set_string_value(src, KEY, "/w").unwrap();
        assert!(out.contains("\n\t\"claudeCode.claudeProcessWrapper\""));
    }

    #[test]
    fn a_windows_path_is_escaped_not_pasted() {
        let out = set_string_value("{}", KEY, r"C:\Users\me\claude-vscode.cmd").unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(parsed[KEY], r"C:\Users\me\claude-vscode.cmd");
    }

    #[test]
    fn removing_the_last_entry_leaves_valid_json() {
        let src =
            "{\n    \"editor.fontSize\": 13,\n    \"claudeCode.claudeProcessWrapper\": \"/w\"\n}\n";
        let out = remove_key(src, KEY).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(parsed["editor.fontSize"], 13);
        assert!(parsed.get(KEY).is_none());
    }

    #[test]
    fn removing_a_middle_entry_leaves_valid_json() {
        let src =
            "{\n    \"claudeCode.claudeProcessWrapper\": \"/w\",\n    \"editor.fontSize\": 13\n}\n";
        let out = remove_key(src, KEY).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(parsed["editor.fontSize"], 13);
        assert!(parsed.get(KEY).is_none());
    }

    #[test]
    fn removing_the_only_entry_leaves_an_empty_object() {
        let src = "{\n    \"claudeCode.claudeProcessWrapper\": \"/w\"\n}\n";
        let out = remove_key(src, KEY).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert!(parsed.as_object().unwrap().is_empty());
    }

    #[test]
    fn removing_keeps_the_comments_around_it() {
        let src = "{\n    // mine\n    \"editor.fontSize\": 13,\n    \"claudeCode.claudeProcessWrapper\": \"/w\"\n}\n";
        let out = remove_key(src, KEY).unwrap();
        assert!(out.contains("// mine"));
    }

    #[test]
    fn removing_an_absent_key_reports_nothing_to_do() {
        assert!(remove_key("{\n    \"editor.fontSize\": 13\n}\n", KEY).is_none());
    }

    #[test]
    fn a_value_containing_braces_or_quotes_does_not_confuse_the_scan() {
        let src = r#"{ "a": "}\"{", "claudeCode.claudeProcessWrapper": "/w", "b": [1, {"c": 2}] }"#;
        assert_eq!(read_string_value(src, KEY), Some("/w".to_string()));
        let out = remove_key(src, KEY).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(parsed["a"], "}\"{");
        assert_eq!(parsed["b"][1]["c"], 2);
    }

    #[test]
    fn a_truncated_file_is_refused_rather_than_panicking() {
        // Regression: an unterminated key string made `skip_string` return
        // `b.len()`, and `key_end - 1` then ran backwards past the slice
        // start or landed inside a multi-byte character. Slicing `src` with
        // it panicked — and `claude-acc install` panicked with it, since the
        // VS Code hint goes through here.
        for src in [
            "{\n    \"",
            "{\n    \"editor.fontSize\": 13,\n    \"тем",
            "{\n    \"a\": \"unterminated",
            "{",
            "{\n    \"a\"",
            "{\n    \"a\": ",
        ] {
            assert_eq!(read_string_value(src, KEY), None, "{src:?}");
            assert!(set_string_value(src, KEY, "/w").is_none(), "{src:?}");
            assert!(remove_key(src, KEY).is_none(), "{src:?}");
        }
    }

    #[test]
    fn removing_the_last_entry_does_not_cut_into_a_preceding_comment() {
        // Regression: the trailing-comma cleanup searched backwards through
        // raw text and found the comma inside `// dark, light, high contrast,`
        // — excising from mid-comment to the end of our entry, closing brace
        // included. VS Code then rejects the file and resets every setting.
        let src = "{\n    \"editor.fontSize\": 13,\n    \"workbench.colorTheme\": \"Default Dark+\", // dark, light, high contrast,\n    \"claudeCode.claudeProcessWrapper\": \"/w\"\n}\n";
        let out = remove_key(src, KEY).unwrap();

        assert!(
            out.contains("// dark, light, high contrast,"),
            "comment was cut: {out}"
        );
        assert!(out.trim_end().ends_with('}'), "closing brace lost: {out}");
        let stripped: String = out
            .lines()
            .map(|l| match l.find("//") {
                Some(i) => &l[..i],
                None => l,
            })
            .collect::<Vec<_>>()
            .join("\n");
        let parsed: serde_json::Value = serde_json::from_str(&stripped).unwrap();
        assert!(parsed.get(KEY).is_none());
        assert_eq!(parsed["editor.fontSize"], 13);
    }

    #[test]
    fn a_standalone_comment_before_the_last_entry_survives_its_removal() {
        let src = "{\n    \"editor.fontSize\": 13,\n    // TODO: fonts, themes,\n    \"claudeCode.claudeProcessWrapper\": \"/w\"\n}\n";
        let out = remove_key(src, KEY).unwrap();
        assert!(out.contains("// TODO: fonts, themes,"), "{out}");
        assert!(out.trim_end().ends_with('}'), "{out}");
    }

    #[test]
    fn no_truncation_of_a_realistic_file_can_panic_or_corrupt_it() {
        // The scanner is hand-rolled byte-level parsing over text that
        // arrives half-written after a crash or a sync race. Walk every
        // prefix, including ones cutting through a multi-byte character.
        let full = "{\n    // тема\n    \"workbench.colorTheme\": \"Тёмная\",\n    \"claudeCode.claudeProcessWrapper\": \"/путь/claude-vscode\",\n    \"editor.fontSize\": 13\n}\n";
        for cut in 0..=full.len() {
            let Some(src) = full.get(..cut) else {
                continue; // mid-character; `get` declines rather than panics
            };
            let _ = read_string_value(src, KEY);
            let _ = remove_key(src, KEY);
            if let Some(out) = set_string_value(src, KEY, "/w") {
                // Anything it agrees to write must come back readable.
                assert_eq!(
                    read_string_value(&out, KEY).as_deref(),
                    Some("/w"),
                    "{src:?}"
                );
            }
        }
    }

    #[test]
    fn set_then_remove_returns_the_file_to_where_it_started() {
        let src = "{\n    // keep\n    \"editor.fontSize\": 13\n}\n";
        let with = set_string_value(src, KEY, "/w").unwrap();
        let without = remove_key(&with, KEY).unwrap();
        assert_eq!(without, src);
    }

    fn scratch(what: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("cc-vscode-{}-{}", what, std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    #[cfg(not(windows))]
    fn the_installed_wrapper_is_executable_and_names_this_binary() {
        // Regression: the wrapper embeds the claude-acc path, so `update`
        // rewrites it. A wrapper left with the placeholder in it would
        // silently fall back to the default account on every launch.
        use std::os::unix::fs::PermissionsExt;
        let dir = scratch("wrapper");
        let bin = dir.join("claude-acc");

        let w = install_wrapper(&dir, &bin).unwrap();
        assert_eq!(w, wrapper_path(&dir));
        let body = fs::read_to_string(&w).unwrap();
        assert!(body.contains(&bin.display().to_string()));
        assert!(!body.contains(WRAPPER_PLACEHOLDER));
        assert_eq!(
            fs::metadata(&w).unwrap().permissions().mode() & 0o111,
            0o111
        );

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_settings_file_that_does_not_exist_yet_reads_as_unset_and_is_created() {
        let dir = scratch("absent");
        let settings = dir.join("User").join("settings.json");
        let ours = dir.join("bin").join(WRAPPER_NAME);

        assert!(matches!(
            wrapper_state(&settings, &ours),
            WrapperState::Unset
        ));
        assert!(set_wrapper(&settings, &ours).unwrap());
        assert!(matches!(
            wrapper_state(&settings, &ours),
            WrapperState::Ours
        ));

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_settings_file_that_is_not_valid_utf8_is_never_mistaken_for_an_absent_one() {
        // Regression: read_to_string collapses "no file" with "unreadable
        // file", so a settings.json holding one Latin-1 byte read as empty
        // and set_wrapper replaced the whole editor config with an object
        // containing only our key — exit 0, success message, config gone.
        let dir = scratch("nonutf8");
        let settings = dir.join("settings.json");
        let ours = dir.join("bin").join(WRAPPER_NAME);
        let original: &[u8] = b"{\n    \"workbench.colorTheme\": \"Caf\xe9 Noir\"\n}\n";
        fs::write(&settings, original).unwrap();

        assert!(matches!(
            wrapper_state(&settings, &ours),
            WrapperState::Unreadable
        ));
        assert!(set_wrapper(&settings, &ours).is_err());
        assert!(clear_wrapper(&settings).is_err());
        assert_eq!(
            fs::read(&settings).unwrap(),
            original,
            "the file was rewritten"
        );

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn a_settings_file_we_cannot_read_is_reported_rather_than_replaced() {
        use std::os::unix::fs::PermissionsExt;
        let dir = scratch("noperm");
        let settings = dir.join("settings.json");
        let ours = dir.join("bin").join(WRAPPER_NAME);
        fs::write(&settings, "{\n    \"editor.fontSize\": 13\n}\n").unwrap();
        fs::set_permissions(&settings, fs::Permissions::from_mode(0o000)).unwrap();

        // Running as root defeats the permission bits entirely; skip rather
        // than assert something the environment can't produce.
        if fs::read_to_string(&settings).is_ok() {
            fs::set_permissions(&settings, fs::Permissions::from_mode(0o644)).unwrap();
            fs::remove_dir_all(&dir).unwrap();
            return;
        }

        assert!(matches!(
            wrapper_state(&settings, &ours),
            WrapperState::Unreadable
        ));
        assert!(set_wrapper(&settings, &ours).is_err());

        fs::set_permissions(&settings, fs::Permissions::from_mode(0o644)).unwrap();
        assert_eq!(
            fs::read_to_string(&settings).unwrap(),
            "{\n    \"editor.fontSize\": 13\n}\n"
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_wrapper_belonging_to_something_else_is_reported_as_foreign() {
        let dir = scratch("foreign");
        let settings = dir.join("settings.json");
        let ours = dir.join("bin").join(WRAPPER_NAME);
        fs::write(
            &settings,
            format!(
                "{{\n    \"{}\": \"/opt/other/launcher\"\n}}\n",
                WRAPPER_SETTING
            ),
        )
        .unwrap();

        match wrapper_state(&settings, &ours) {
            WrapperState::Foreign(v) => assert_eq!(v, "/opt/other/launcher"),
            _ => panic!("expected Foreign"),
        }

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_settings_file_that_is_not_an_object_is_reported_rather_than_written_over() {
        let dir = scratch("unreadable");
        let settings = dir.join("settings.json");
        let ours = dir.join("bin").join(WRAPPER_NAME);
        fs::write(&settings, "[\"not an object\"]").unwrap();

        assert!(matches!(
            wrapper_state(&settings, &ours),
            WrapperState::Unreadable
        ));
        assert!(!set_wrapper(&settings, &ours).unwrap());
        assert_eq!(
            fs::read_to_string(&settings).unwrap(),
            "[\"not an object\"]"
        );

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn clearing_a_file_that_never_had_the_setting_reports_nothing_to_do() {
        let dir = scratch("clear");
        let settings = dir.join("settings.json");
        fs::write(&settings, "{\n    \"editor.fontSize\": 13\n}\n").unwrap();

        assert!(!clear_wrapper(&settings).unwrap());
        assert!(!clear_wrapper(&dir.join("nope.json")).unwrap());

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_generated_wrapper_execs_the_binary_the_extension_passes() {
        // Regression: the PATH wrapper searches PATH and forwards all of
        // "$@". Under this calling convention that hands the real binary
        // its own path as an argument.
        #[cfg(not(windows))]
        {
            assert!(VSCODE_WRAPPER_TEMPLATE.contains("real=\"$1\""));
            assert!(VSCODE_WRAPPER_TEMPLATE.contains("exec \"$real\" \"$@\""));
            // And it must re-derive the account rather than trust an
            // inherited value — the extension host's environment is the
            // login shell's, resolved once, for the home directory.
            assert!(!VSCODE_WRAPPER_TEMPLATE.contains("-z \"$CLAUDE_CONFIG_DIR\""));
            // Regression: without the unset, an `activate` that fails leaves
            // the inherited value standing, which is the one thing the
            // wrapper exists to stop trusting.
            let unset = VSCODE_WRAPPER_TEMPLATE.find("unset CLAUDE_CONFIG_DIR");
            let eval = VSCODE_WRAPPER_TEMPLATE.find("activate --shell posix");
            assert!(
                unset.is_some() && unset < eval,
                "unset must precede the eval"
            );
        }
    }

    #[test]
    fn settings_path_ends_where_vs_code_keeps_it() {
        let p = settings_path("Code").unwrap();
        assert!(p.ends_with("Code/User/settings.json") || p.ends_with(r"Code\User\settings.json"));
    }
}
