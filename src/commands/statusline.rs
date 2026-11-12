use std::io::Read;
use std::path::Path;
use std::process::Command;

use serde_json::Value;

use crate::commands::install::binary_name;
use crate::config::AppConfig;
use crate::i18n::{I18n, Msg};
use crate::resolve;

// `claude-acc statusline` is meant to be wired into Claude Code's `statusLine`
// setting. Claude Code pipes session JSON on stdin (model, workspace, git repo,
// and the live `context_window` usage), and renders whatever we print, ANSI
// colors included. We add the one thing Claude Code can't know: which managed
// account this session is running under (from CLAUDE_CONFIG_DIR).
//
// `--install` writes the `statusLine` block into the active account's
// settings.json so the user doesn't have to hand-edit JSON.

const BAR_WIDTH: usize = 10;

pub fn run(config: &AppConfig, i18n: &I18n, install: bool) -> i32 {
    if install {
        return install_into_settings(config, i18n);
    }
    render(config);
    0
}

fn render(config: &AppConfig) {
    let mut input = String::new();
    let _ = std::io::stdin().read_to_string(&mut input);
    let v: Value = serde_json::from_str(&input).unwrap_or(Value::Null);

    let mut segs: Vec<String> = Vec::new();

    if let Some(acc) = account_label(config) {
        // Bold cyan badge — the account is the headline of this status line.
        segs.push(paint("1;36", &acc));
    }
    if let Some(branch) = git_branch(&v) {
        segs.push(paint("32", &format!("⎇ {}", branch)));
    }
    if let Some(model) = v.pointer("/model/display_name").and_then(Value::as_str) {
        segs.push(model.to_string());
    }
    if let Some(project) = project_name(&v) {
        segs.push(paint("33", &project));
    }
    if let Some(seg) = context_segment(&v) {
        segs.push(seg);
    }

    if segs.is_empty() {
        return;
    }
    let sep = format!(" {} ", paint("90", "│"));
    println!("{}", segs.join(&sep));
}

/// Account name for the current session, from `CLAUDE_CONFIG_DIR`:
/// `<name>` for a managed account, `default` for the standard `~/.claude/`
/// (or when the variable is unset), else the dir's basename.
fn account_label(config: &AppConfig) -> Option<String> {
    let Some(ccd) = std::env::var_os("CLAUDE_CONFIG_DIR") else {
        return Some("default".to_string());
    };
    let p = Path::new(&ccd);
    if let Ok(rel) = p.strip_prefix(config.accounts_dir())
        && let Some(first) = rel.components().next()
        && let Some(name) = first.as_os_str().to_str()
    {
        return Some(name.to_string());
    }
    if let Some(home) = dirs::home_dir()
        && p == home.join(".claude")
    {
        return Some("default".to_string());
    }
    p.file_name()
        .and_then(|n| n.to_str())
        .map(String::from)
        .or(Some("default".to_string()))
}

fn git_branch(v: &Value) -> Option<String> {
    let cwd = cwd_of(v)?;
    let out = Command::new("git")
        .args(["-C", cwd, "branch", "--show-current"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let branch = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if branch.is_empty() {
        None
    } else {
        Some(branch)
    }
}

fn project_name(v: &Value) -> Option<String> {
    let dir = v
        .pointer("/workspace/project_dir")
        .and_then(Value::as_str)
        .or_else(|| cwd_of(v))?;
    Path::new(dir)
        .file_name()
        .and_then(|n| n.to_str())
        .map(String::from)
}

fn cwd_of(v: &Value) -> Option<&str> {
    v.pointer("/workspace/current_dir")
        .and_then(Value::as_str)
        .or_else(|| v.get("cwd").and_then(Value::as_str))
}

/// The margin this meter keeps back for auto-compaction, as a share of the
/// window, for the colour only.
///
/// It is our own figure, not Claude Code's. Claude Code keeps back a token
/// count, not a share: in 2.1.322 the compaction trigger sits at the window
/// less `min(maxOutputTokens, 20000)`, and `maxOutputTokens` is not in the
/// status line payload. So 16.5% is a stand-in — about right on a 200k
/// window, far too cautious on a 1M one, where it turns the bar red with a
/// third of the window still free. Worth replacing with the real reserve;
/// that is a change to the colour, not to the digits, and belongs in its own
/// change with its own measurements.
const AUTO_COMPACT_BUFFER_PCT: f64 = 16.5;

/// The range Claude Code accepts for an auto-compact window, in tokens.
///
/// Transcribed from Claude Code 2.1.322, the `modelSettings.autoCompactWindow`
/// settings description: "Auto-compact window for this model, in tokens
/// (100000 to 1000000), or \"auto\" for the window tuned for the model."
/// Anything outside it is reported as invalid and ignored, so it is ignored
/// here too rather than clamped.
const AUTO_COMPACT_WINDOW_MIN: f64 = 100_000.0;
const AUTO_COMPACT_WINDOW_MAX: f64 = 1_000_000.0;

/// What `autoCompactWindow` says in one settings file.
#[derive(Debug, PartialEq)]
enum WindowSetting {
    /// A window in tokens.
    Tokens(f64),
    /// `"auto"` — the window tuned for the model, i.e. no override. It
    /// *replaces* a top-level value for that model rather than falling
    /// through to it.
    Auto,
}

/// Context-window usage segment from Claude Code's `context_window` block.
///
/// The number shown is the used percentage of the window this session is
/// actually held to: the model's window, or the smaller `autoCompactWindow`
/// when one is configured.
///
/// `context_window` in the payload is always the **model's** window — Claude
/// Code resolves the auto-compact window separately and sends neither it nor
/// a percentage against it (read out of 2.1.322: the payload's
/// `context_window` is built from the model-window resolver, which honours
/// only `CLAUDE_CODE_MAX_CONTEXT_TOKENS`, and then only under
/// `DISABLE_COMPACT`). So a session capped at 400k on a 1M model reported 39%
/// here while Claude Code's own footer — which counts against the resolved
/// window — was nearly full. That is what this scaling fixes.
///
/// It did not used to scale at all. The percentage was once taken against the
/// *usable* window, with the auto-compaction reserve removed, which left two
/// numbers for one quantity with no way to tell which was which. Agreeing
/// with the application wins; the reserve still decides the *colour*, which is
/// where it carries real meaning — red keeps saying "compaction is about to
/// kick in" rather than an arbitrary fraction of the window. So the digits
/// answer "how full is this conversation" and the colour answers "how close is
/// the wall".
fn context_segment(v: &Value) -> Option<String> {
    let remaining = v
        .pointer("/context_window/remaining_percentage")
        .and_then(Value::as_f64)?;
    // `context_window_size`, not `total_tokens` — the latter has never been a
    // field Claude Code sends, so this read always missed and silently fell
    // back to the default below.
    let total = v
        .pointer("/context_window/context_window_size")
        .and_then(Value::as_f64)
        .filter(|t| *t > 0.0)
        .unwrap_or(1_000_000.0);

    let effective = effective_window(total, configured_window(v));
    let shown = used_share(used_tokens(v, remaining, total), effective);
    Some(usage_segment(shown, used_of_usable(100.0 - shown)))
}

/// Tokens in the context window right now.
///
/// `total_input_tokens` is the exact count, and it is the same sum Claude
/// Code divides by the window for the percentages beside it — `input_tokens +
/// cache_creation_input_tokens + cache_read_input_tokens` (2.1.322). The
/// percentage is that ratio **rounded to a whole number**, so recovering
/// tokens from it costs up to half a point of the window — 5k on a 1M window
/// — and scaling to a smaller window multiplies that error by
/// `total / effective`. So the token count is preferred, and the percentage
/// stays only as the fallback for a payload without it.
fn used_tokens(v: &Value, remaining: f64, total: f64) -> f64 {
    v.pointer("/context_window/total_input_tokens")
        .and_then(Value::as_f64)
        .filter(|t| t.is_finite() && *t >= 0.0)
        .unwrap_or_else(|| (100.0 - remaining).clamp(0.0, 100.0) / 100.0 * total)
}

/// The window this session is held to: the configured auto-compact window,
/// never larger than the model's own window (Claude Code clamps it the same
/// way), and the model's window when nothing is configured.
fn effective_window(total: f64, configured: Option<f64>) -> f64 {
    match configured {
        Some(w) if w > 0.0 => w.min(total),
        _ => total,
    }
}

/// Used share of `window`, 0–100.
fn used_share(used: f64, window: f64) -> f64 {
    if window <= 0.0 {
        return 0.0;
    }
    (used / window * 100.0).clamp(0.0, 100.0)
}

/// How much of the *usable* window is spent, 0–100 — the window minus the
/// slice Claude Code keeps back for the compaction summary.
///
/// `remaining` is the share of the effective window still free.
fn used_of_usable(remaining: f64) -> f64 {
    let usable_remaining =
        (((remaining - AUTO_COMPACT_BUFFER_PCT) / (100.0 - AUTO_COMPACT_BUFFER_PCT)) * 100.0)
            .max(0.0);
    (100.0 - usable_remaining).clamp(0.0, 100.0)
}

/// The auto-compact window configured for this session, in tokens.
///
/// Precedence, transcribed from Claude Code 2.1.322's resolver:
/// `CLAUDE_CODE_AUTO_COMPACT_WINDOW` first — "`CLAUDE_CODE_AUTO_COMPACT_WINDOW`
/// is set and takes precedence" is what `/autocompact` says when it is — then
/// the `autoCompactWindow` setting, `modelSettings.<model>.autoCompactWindow`
/// ahead of the top-level key within one file.
///
/// The environment variable is a **window** in tokens, not a reserve: Claude
/// Code's own advice for enforcing a smaller window reads "set
/// `CLAUDE_CODE_AUTO_COMPACT_WINDOW=<tokens>` (or the `autoCompactWindow`
/// setting)". It used to be read here as a reserve to subtract, which made a
/// 400k setting look like a 40% reserve on a 1M window.
///
/// Not read: an organisation's managed (policy) settings, and the
/// server-side and experiment defaults below the settings in Claude Code's
/// order. A window lowered by one of those is not reflected in the meter.
fn configured_window(v: &Value) -> Option<f64> {
    let model = v
        .pointer("/model/id")
        .and_then(Value::as_str)
        .unwrap_or_default();

    if let Ok(raw) = std::env::var("CLAUDE_CODE_AUTO_COMPACT_WINDOW")
        && let Some(tokens) = env_window(&raw)
    {
        return Some(tokens);
    }

    let docs: Vec<Value> = settings_files(v)
        .into_iter()
        .filter_map(|p| std::fs::read_to_string(p).ok())
        .filter_map(|s| serde_json::from_str::<Value>(&s).ok())
        .collect();
    window_from_docs(&docs, model)
}

/// Claude Code's settings files, highest precedence first: the project's local
/// settings, the project's shared settings, then the account's own
/// `settings.json` — `CLAUDE_CONFIG_DIR` for a managed account, `~/.claude`
/// otherwise, which is the same file `statusline --install` writes to.
fn settings_files(v: &Value) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    if let Some(project) = v
        .pointer("/workspace/project_dir")
        .and_then(Value::as_str)
        .or_else(|| cwd_of(v))
    {
        let dir = Path::new(project).join(".claude");
        out.push(dir.join("settings.local.json"));
        out.push(dir.join("settings.json"));
    }
    let user = std::env::var_os("CLAUDE_CONFIG_DIR")
        .map(std::path::PathBuf::from)
        .or_else(|| dirs::home_dir().map(|h| h.join(".claude")));
    if let Some(user) = user {
        out.push(user.join("settings.json"));
    }
    out
}

/// The first of `docs` that configures a window for `model_id` decides, in the
/// order given. A model block saying `"auto"` decides too — it means this
/// model keeps its own window, and does not fall through to a top-level value.
fn window_from_docs(docs: &[Value], model_id: &str) -> Option<f64> {
    for doc in docs {
        match settings_window(doc, model_id) {
            Some(WindowSetting::Tokens(t)) => return Some(t),
            Some(WindowSetting::Auto) => return None,
            None => continue,
        }
    }
    None
}

/// `autoCompactWindow` out of one settings document, for `model_id`.
fn settings_window(doc: &Value, model_id: &str) -> Option<WindowSetting> {
    if let Some(blocks) = doc.pointer("/modelSettings").and_then(Value::as_object) {
        for (key, block) in blocks {
            if model_key_matches(key, model_id)
                && let Some(value) = block.get("autoCompactWindow")
            {
                return window_setting(value);
            }
        }
    }
    doc.get("autoCompactWindow").and_then(window_setting)
}

/// One `autoCompactWindow` value: a token count in range, `"auto"`, or
/// nothing at all for a value Claude Code would reject.
fn window_setting(value: &Value) -> Option<WindowSetting> {
    if value.as_str() == Some("auto") {
        return Some(WindowSetting::Auto);
    }
    let tokens = value.as_f64()?;
    if (AUTO_COMPACT_WINDOW_MIN..=AUTO_COMPACT_WINDOW_MAX).contains(&tokens) {
        Some(WindowSetting::Tokens(tokens))
    } else {
        None
    }
}

/// `CLAUDE_CODE_AUTO_COMPACT_WINDOW` read the way Claude Code reads it.
///
/// Transcribed from 2.1.322: a trailing `k` or `m` multiplies by 1000 or
/// 1000000; under one token, or unparseable, it is ignored — "isn't a usable
/// size, so Claude Code ignores it" — and outside the 100000..=1000000 bounds
/// it is **clamped** to the nearer bound rather than dropped.
///
/// Reading it as a plain integer, as this did first, ignored `400k`
/// altogether and took `1` for a one-token window, which pinned the meter at
/// 100% on an empty context.
///
/// A settings value is deliberately not run through this: Claude Code
/// validates those against the same bounds when it loads the file and drops
/// an out-of-range one, where the environment variable is clamped. The
/// asymmetry is theirs.
fn env_window(raw: &str) -> Option<f64> {
    let s = raw.trim();
    let (digits, scale) = match s.chars().last()?.to_ascii_lowercase() {
        'm' => (&s[..s.len() - 1], 1_000_000.0),
        'k' => (&s[..s.len() - 1], 1_000.0),
        _ => (s, 1.0),
    };
    let tokens = digits.trim().parse::<f64>().ok()? * scale;
    if !tokens.is_finite() || tokens < 1.0 {
        return None;
    }
    Some(
        tokens
            .round()
            .clamp(AUTO_COMPACT_WINDOW_MIN, AUTO_COMPACT_WINDOW_MAX),
    )
}

/// Whether a `modelSettings` key addresses the model this session runs on.
///
/// Claude Code: "The canonical model name as key also matches its dated,
/// [1m], Bedrock and Vertex spellings." So `claude-opus-5` has to match
/// `claude-opus-5[1m]`, `claude-opus-5-20260301`,
/// `us.anthropic.claude-opus-5-v1:0` and `claude-opus-5@20260301`, while
/// staying off `claude-opus-5-5`, which is a different model.
fn model_key_matches(key: &str, model_id: &str) -> bool {
    key == model_id || canonical_model(key) == canonical_model(model_id)
}

/// A model id reduced to its canonical name.
fn canonical_model(id: &str) -> String {
    let mut s = id.trim();
    // `claude-opus-5[1m]`, and Vertex's `claude-opus-5@20260301`.
    if let Some(i) = s.find(['[', '@']) {
        s = &s[..i];
    }
    // Bedrock's `us.anthropic.claude-opus-5-v1:0`.
    if let Some(i) = s.rfind('.') {
        s = &s[i + 1..];
    }
    let mut out = s.to_string();
    // ... and its `-v1:0` revision suffix.
    if let Some(i) = out.rfind("-v")
        && out[i + 2..].chars().all(|c| c.is_ascii_digit() || c == ':')
        && out[i + 2..]
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_digit())
    {
        out.truncate(i);
    }
    // A dated spelling: `-20260301`. Truncating only an eight-digit tail keeps
    // `claude-opus-5-5` from collapsing onto `claude-opus-5`.
    if let Some(i) = out.rfind('-')
        && out[i + 1..].len() == 8
        && out[i + 1..].chars().all(|c| c.is_ascii_digit())
    {
        out.truncate(i);
    }
    out
}

/// A coloured 10-cell bar plus percentage, at a width that never changes.
///
/// `shown` is what the digits and the bar report — the used share of the whole
/// window, matching Claude Code. `urgency` is the same session measured
/// against the usable window, and only picks the colour.
///
/// The segment is a fixed 15 columns because this is the last thing on the
/// line and anything that changes its width moves everything. Two things used
/// to: a `💀 ` was prepended at the severe threshold, so crossing it shifted
/// the line by two columns — more in terminals that render the emoji
/// double-width, and they disagree, so with a redraw every few hundred
/// milliseconds it read as the line drifting. And the percentage was
/// unpadded, so it moved again between `99%` and `100%`. Measured: 14 columns
/// below the threshold, 16 above it, 17 at full.
///
/// Severity is carried by colour alone now. It is already the thing the eye
/// catches, and it costs no layout.
fn usage_segment(shown: f64, urgency: f64) -> String {
    let (code, body) = usage_parts(shown, urgency);
    paint(code, &body)
}

/// The SGR code and the text, before any painting.
///
/// Split out so that "the digits come from `shown` and the colour from
/// `urgency`" is assertable without `NO_COLOR` — a test that sets or clears
/// that variable passes alone and then fails inside the full suite, which is
/// exactly what the first version of this one did.
fn usage_parts(shown: f64, urgency: f64) -> (&'static str, String) {
    let p = shown.clamp(0.0, 100.0);
    let filled = ((p / 100.0) * BAR_WIDTH as f64).floor() as usize;
    let filled = filled.min(BAR_WIDTH);
    let bar = format!("{}{}", "▓".repeat(filled), "░".repeat(BAR_WIDTH - filled));
    // Right-aligned in a 4-wide field: "  5%", " 47%", "100%" all measure the
    // same, so the bar never shifts as the number grows a digit.
    let body = format!("{} {:>4}", bar, format!("{}%", p.round() as i64));
    (severity_code(urgency), body)
}

/// The SGR code for a 0–100 used value.
///
/// Split from the rendering so the choice can be asserted without `NO_COLOR`,
/// which the other tests here mutate.
///
/// Takes the urgency value — the session measured against the *usable*
/// window — not the percentage on screen, so "red" still means compaction is
/// imminent rather than an arbitrary fraction of the raw window.
///
/// Bold red at the top, not inverse and not blink. Blink was the original
/// choice and never worked: Claude Code parses SGR 5 and discards it, keeping
/// only colour, dim, bold, italic, underline, strikethrough and inverse.
/// Inverse replaced it and was worse — it fills the whole segment, background
/// included, so the bar stops reading as a bar and the block appears to swim
/// against its neighbours. Bold is emphasis without repainting the cell.
fn severity_code(pct: f64) -> &'static str {
    if pct >= 80.0 {
        "1;31" // bold red — compaction is about to kick in
    } else if pct >= 65.0 {
        "38;5;208" // orange
    } else if pct >= 50.0 {
        "33" // yellow
    } else {
        "32" // green
    }
}

/// Wrap `text` in an ANSI SGR sequence, unless `NO_COLOR` is set.
fn paint(code: &str, text: &str) -> String {
    if std::env::var_os("NO_COLOR").is_some() {
        text.to_string()
    } else {
        format!("\x1b[{}m{}\x1b[0m", code, text)
    }
}

/// Render `bin` for the `statusLine.command` string.
///
/// Claude Code runs the status line command through a shell — on Windows that
/// shell is Git Bash (`/bin/bash.exe`), where a backslash is an escape
/// character, so a native `C:\Users\...` path collapses to `C:Users...` and the
/// binary is never found (the status line silently renders blank). A
/// forward-slash path (`C:/Users/...`) is understood by both Git Bash and the
/// Windows API, so it is safe regardless of which shell runs the command.
fn command_path(bin: &Path) -> String {
    let s = bin.display().to_string();
    if cfg!(windows) {
        s.replace('\\', "/")
    } else {
        s
    }
}

/// The settings object to add `statusLine` to: empty when there is no file
/// yet, the file's own object otherwise. `None` for a file that exists but is
/// not a JSON object, or cannot be read: writing over it would replace
/// everything in it with the one key this command adds.
fn settings_root(read: std::io::Result<String>) -> Option<Value> {
    match read {
        Ok(text) => serde_json::from_str::<Value>(&text)
            .ok()
            .filter(Value::is_object),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Some(serde_json::json!({})),
        Err(_) => None,
    }
}

fn install_into_settings(config: &AppConfig, i18n: &I18n) -> i32 {
    let cwd = std::env::current_dir().unwrap_or_default();
    // The account this directory currently resolves to (managed account or the
    // standard ~/.claude when there's no link / default).
    let (label, settings_dir) = match resolve::resolve_account(config, &cwd) {
        Some(name) => (name.clone(), config.account_path(&name)),
        None => (
            "default".to_string(),
            dirs::home_dir().unwrap_or_default().join(".claude"),
        ),
    };

    if let Err(e) = std::fs::create_dir_all(&settings_dir) {
        i18n.print(Msg::StatuslineInstallFailed(e.to_string()));
        return 1;
    }
    let settings_path = settings_dir.join("settings.json");

    let Some(mut root) = settings_root(std::fs::read_to_string(&settings_path)) else {
        i18n.print(Msg::StatuslineSettingsUnreadable(
            settings_path.display().to_string(),
        ));
        return 1;
    };

    let bin = config.base_dir.join("bin").join(binary_name());
    root["statusLine"] = serde_json::json!({
        "type": "command",
        "command": format!("{} statusline", command_path(&bin)),
        "padding": 0,
    });

    let serialized = match serde_json::to_string_pretty(&root) {
        Ok(s) => s,
        Err(e) => {
            i18n.print(Msg::StatuslineInstallFailed(e.to_string()));
            return 1;
        }
    };
    if let Err(e) = std::fs::write(&settings_path, serialized) {
        i18n.print(Msg::StatuslineInstallFailed(e.to_string()));
        return 1;
    }

    i18n.print(Msg::StatuslineInstalled(
        label,
        settings_path.display().to_string(),
    ));
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_missing_file_or_a_json_object_is_edited() {
        // Regression: a settings.json that did not parse was replaced by
        // `{"statusLine": …}`, and everything else in it was lost.
        let read = |text: &str| Ok(text.to_string());
        assert_eq!(
            settings_root(read(r#"{"theme":"dark"}"#)),
            Some(serde_json::json!({"theme": "dark"}))
        );
        assert_eq!(
            settings_root(Err(std::io::ErrorKind::NotFound.into())),
            Some(serde_json::json!({}))
        );
        for broken in [r#"{"theme": "dark","#, "[]", "\"text\"", ""] {
            assert_eq!(settings_root(read(broken)), None, "{broken:?}");
        }
        assert_eq!(
            settings_root(Err(std::io::ErrorKind::PermissionDenied.into())),
            None
        );
    }

    #[test]
    fn usage_segment_includes_rounded_percent() {
        // NO_COLOR keeps the assertion free of escape codes.
        unsafe { std::env::set_var("NO_COLOR", "1") };
        let seg = usage_segment(32.4, 32.4);
        assert!(seg.contains("32%"), "got {seg:?}");
        assert!(seg.contains('▓') && seg.contains('░'), "got {seg:?}");
    }

    // The number on screen is the used share of the whole window, so it
    // agrees with what Claude Code reports for the same session. It used to
    // be scaled to the usable window instead: a session at 393.5k of 1M read
    // as 39% in the application and 47% here, two numbers for one quantity.
    #[test]
    fn the_percentage_shown_is_the_one_the_application_shows() {
        let v: Value = serde_json::from_str(
            r#"{"context_window":{"remaining_percentage":60.65,"context_window_size":1000000}}"#,
        )
        .unwrap();
        unsafe { std::env::set_var("NO_COLOR", "1") };
        let seg = context_segment(&v).expect("a segment");
        assert!(seg.contains("39%"), "got {seg:?}");
        assert!(!seg.contains("47%"), "the old normalised figure: {seg:?}");
    }

    // Severity is still measured against the usable window, so red keeps
    // meaning "compaction is imminent" rather than a share of the raw window.
    // Those are different numbers for the same session, which is the whole
    // point of passing two values.
    #[test]
    fn the_colour_follows_urgency_while_the_digits_follow_the_window() {
        // Raw 70% used, urgency already past the severe threshold — the two
        // inputs disagree on purpose, which is why there are two of them.
        // Asserted through `usage_parts` so this needs no NO_COLOR: the first
        // version cleared it and broke two unrelated tests in the full suite.
        let (code, body) = usage_parts(70.0, 85.0);
        assert_eq!(code, "1;31", "colour must follow urgency");
        assert!(
            body.contains("70%"),
            "digits must be the raw figure: {body:?}"
        );

        // And the other way round: a high raw figure with slack left stays
        // green rather than taking its colour from the digits.
        let (code, body) = usage_parts(70.0, 10.0);
        assert_eq!(code, "32");
        assert!(body.contains("70%"), "{body:?}");
    }

    // The segment is the last thing on the line, so any width change moves
    // everything to its left. Measured before the fix: 14 columns below the
    // severe threshold, 16 above it (a `💀 ` was prepended), 17 at 100%.
    #[test]
    fn the_segment_width_never_changes() {
        let widths: std::collections::BTreeSet<usize> =
            [0.0, 5.0, 39.0, 66.0, 67.0, 79.0, 80.0, 95.0, 99.4, 100.0]
                .into_iter()
                .map(|p| usage_parts(p, p).1.chars().count())
                .collect();
        assert_eq!(
            widths.len(),
            1,
            "the segment changes width across the range: {widths:?}"
        );
    }

    // A width that is stable only while the colour is stable would still
    // drift, since severity and width used to change together.
    #[test]
    fn crossing_the_severe_threshold_does_not_change_the_width() {
        let below = usage_parts(66.0, 79.9).1.chars().count();
        let above = usage_parts(67.0, 80.0).1.chars().count();
        assert_eq!(below, above);
    }

    // `CLAUDE_CODE_AUTO_COMPACT_WINDOW` and `autoCompactWindow` are a window
    // in tokens, not a reserve to subtract. Read as a reserve, a 400k setting
    // on a 1M model became a 40% reserve and the digits still counted against
    // the full 1M: a session held to 400k, and all but out of room, showed a
    // bar a third full.
    #[test]
    fn a_lowered_auto_compact_window_is_what_the_meter_counts_against() {
        // 393.5k of a 1M model window, but the session is held to 400k, so it
        // is all but full: 393.5 / 400 = 98.375%.
        let effective = effective_window(1_000_000.0, Some(400_000.0));
        assert_eq!(effective, 400_000.0);
        let shown = used_share(393_500.0, effective);
        assert!((shown - 98.375).abs() < 0.01, "got {shown}");

        // Nothing configured: the model window stands and the share is
        // Claude Code's own, unscaled.
        let plain = effective_window(1_000_000.0, None);
        assert_eq!(plain, 1_000_000.0);
        assert!((used_share(393_500.0, plain) - 39.35).abs() < 0.01);
    }

    // Claude Code clamps the configured window to the model's own, so a
    // 1M setting on a 200k model is still a 200k session.
    #[test]
    fn a_window_above_the_models_own_is_clamped_to_it() {
        assert_eq!(effective_window(200_000.0, Some(1_000_000.0)), 200_000.0);
        // And a nonsensical one is ignored rather than dividing by it.
        assert_eq!(effective_window(200_000.0, Some(0.0)), 200_000.0);
        assert_eq!(used_share(100_000.0, 0.0), 0.0);
    }

    #[test]
    fn the_shown_share_never_leaves_the_scale() {
        // Past the configured window — the model window is what Claude Code
        // holds the session to, and it is larger than the compaction trigger.
        assert_eq!(used_share(500_000.0, 400_000.0), 100.0);
        // An empty context, and a count that cannot be right.
        assert_eq!(used_share(0.0, 400_000.0), 0.0);
        assert_eq!(used_share(-5.0, 400_000.0), 0.0);
    }

    // `remaining_percentage` is rounded to a whole number before it is sent,
    // so recovering tokens from it and then scaling to a smaller window
    // multiplies the rounding error by total/effective: one point of a 1M
    // window is 10k tokens, which is 2.5 points of a 400k window. The exact
    // count in `total_input_tokens` is the same sum Claude Code divides, so
    // it is used when present.
    #[test]
    fn the_exact_token_count_is_preferred_to_the_rounded_percentage() {
        let exact = serde_json::json!({
            "context_window": {
                "remaining_percentage": 61.0,
                "context_window_size": 1_000_000,
                "total_input_tokens": 393_500,
            }
        });
        assert_eq!(used_tokens(&exact, 61.0, 1_000_000.0), 393_500.0);
        assert!(
            (used_share(used_tokens(&exact, 61.0, 1_000_000.0), 400_000.0) - 98.375).abs() < 0.01
        );

        // Without it, the rounded percentage is all there is: 39% of 1M is
        // 390k, which is 97.5% of a 400k window — a point adrift, and the
        // reason the exact count is preferred.
        let rounded = serde_json::json!({
            "context_window": { "remaining_percentage": 61.0, "context_window_size": 1_000_000 }
        });
        assert_eq!(used_tokens(&rounded, 61.0, 1_000_000.0), 390_000.0);
        assert_eq!(
            used_share(used_tokens(&rounded, 61.0, 1_000_000.0), 400_000.0),
            97.5
        );

        // A nonsense count falls back rather than poisoning the meter.
        let bad = serde_json::json!({
            "context_window": {
                "remaining_percentage": 61.0,
                "context_window_size": 1_000_000,
                "total_input_tokens": -1,
            }
        });
        assert_eq!(used_tokens(&bad, 61.0, 1_000_000.0), 390_000.0);
    }

    // The environment variable is parsed the way Claude Code parses it: a
    // k/m suffix, a clamp to 100000..=1000000, and an unusable size ignored.
    // Read as a plain integer, `400k` was ignored and `1` became a one-token
    // window that pinned the meter at 100% on an empty context.
    #[test]
    fn the_environment_window_is_parsed_the_way_claude_code_parses_it() {
        assert_eq!(env_window("400000"), Some(400_000.0));
        assert_eq!(env_window(" 400000 "), Some(400_000.0));
        assert_eq!(env_window("400k"), Some(400_000.0));
        assert_eq!(env_window("400K"), Some(400_000.0));
        assert_eq!(env_window("0.5m"), Some(500_000.0));
        // Clamped to the bounds, not dropped.
        assert_eq!(env_window("50000"), Some(100_000.0));
        assert_eq!(env_window("1"), Some(100_000.0));
        assert_eq!(env_window("2m"), Some(1_000_000.0));
        // Not a usable size.
        assert_eq!(env_window("0"), None);
        assert_eq!(env_window(""), None);
        assert_eq!(env_window("lots"), None);
        assert_eq!(env_window("k"), None);
        assert_eq!(env_window("-400000"), None);
    }

    #[test]
    fn the_default_reserve_decides_the_colour() {
        // Remaining == the default 16.5% reserve: usable window fully spent.
        assert_eq!(used_of_usable(16.5), 100.0);
        // An empty context reads as nothing used.
        assert_eq!(used_of_usable(100.0), 0.0);
        // Past the trigger it stays pinned at the top of the scale.
        assert_eq!(used_of_usable(0.0), 100.0);
    }

    #[test]
    fn the_highest_precedence_settings_file_decides_the_window() {
        let local = serde_json::json!({ "autoCompactWindow": 400_000 });
        let project = serde_json::json!({ "autoCompactWindow": 600_000 });
        let user = serde_json::json!({ "autoCompactWindow": 900_000 });
        assert_eq!(
            window_from_docs(&[local, project.clone(), user.clone()], "claude-opus-5"),
            Some(400_000.0)
        );
        // A file that says nothing is skipped, not treated as "auto".
        let silent = serde_json::json!({ "model": "opus" });
        assert_eq!(
            window_from_docs(&[silent, project, user], "claude-opus-5"),
            Some(600_000.0)
        );
        assert_eq!(window_from_docs(&[], "claude-opus-5"), None);
    }

    #[test]
    fn a_model_block_replaces_the_top_level_window_for_that_model() {
        let doc = serde_json::json!({
            "autoCompactWindow": 900_000,
            "modelSettings": { "claude-opus-5": { "autoCompactWindow": 400_000 } },
        });
        assert_eq!(
            settings_window(&doc, "claude-opus-5"),
            Some(WindowSetting::Tokens(400_000.0))
        );
        // Another model falls through to the top-level value.
        assert_eq!(
            settings_window(&doc, "claude-sonnet-5"),
            Some(WindowSetting::Tokens(900_000.0))
        );

        // "auto" in a model block means this model keeps its own window — it
        // replaces the top-level value rather than falling through to it.
        let auto = serde_json::json!({
            "autoCompactWindow": 400_000,
            "modelSettings": { "claude-opus-5": { "autoCompactWindow": "auto" } },
        });
        assert_eq!(
            settings_window(&auto, "claude-opus-5"),
            Some(WindowSetting::Auto)
        );
        // It also settles the question for the files below it: a lower-
        // precedence window must not be picked up after an "auto".
        let below = serde_json::json!({ "autoCompactWindow": 500_000 });
        assert_eq!(window_from_docs(&[auto, below], "claude-opus-5"), None);
    }

    // Claude Code: "The canonical model name as key also matches its dated,
    // [1m], Bedrock and Vertex spellings."
    #[test]
    fn a_canonical_key_matches_the_models_other_spellings() {
        for id in [
            "claude-opus-5",
            "claude-opus-5[1m]",
            "claude-opus-5-20260301",
            "claude-opus-5@20260301",
            "us.anthropic.claude-opus-5-v1:0",
        ] {
            assert!(model_key_matches("claude-opus-5", id), "{id}");
        }
        // A different model with a longer name is not the same model.
        assert!(!model_key_matches("claude-opus-5", "claude-opus-5-5"));
        assert!(!model_key_matches("claude-opus-5", "claude-sonnet-5"));
        // And the key may itself be written in one of those spellings.
        assert!(model_key_matches("claude-opus-5[1m]", "claude-opus-5"));
    }

    // Out of Claude Code's accepted 100000..=1000000, a value is reported as
    // invalid and ignored. Ignoring it here keeps the meter on the model
    // window rather than inventing a window Claude Code is not using.
    #[test]
    fn an_out_of_range_or_malformed_window_is_ignored() {
        for bad in [
            serde_json::json!({ "autoCompactWindow": 50_000 }),
            serde_json::json!({ "autoCompactWindow": 2_000_000 }),
            serde_json::json!({ "autoCompactWindow": "400k" }),
            serde_json::json!({ "autoCompactWindow": true }),
            serde_json::json!({ "autoCompactWindow": null }),
        ] {
            assert_eq!(settings_window(&bad, "claude-opus-5"), None, "{bad}");
        }
        // The boundaries themselves are accepted.
        let min = serde_json::json!({ "autoCompactWindow": 100_000 });
        assert_eq!(
            settings_window(&min, "claude-opus-5"),
            Some(WindowSetting::Tokens(100_000.0))
        );
        let max = serde_json::json!({ "autoCompactWindow": 1_000_000 });
        assert_eq!(
            settings_window(&max, "claude-opus-5"),
            Some(WindowSetting::Tokens(1_000_000.0))
        );
    }

    // Two attributes have been wrong here. Blink (SGR 5) never rendered:
    // Claude Code parses it and drops it, keeping only colour, dim, bold,
    // italic, underline, strikethrough and inverse. Inverse then replaced it
    // and survived projection but repainted the whole segment background, so
    // the bar stopped reading as a bar and appeared to swim. Bold is emphasis
    // that leaves the cell alone.
    #[test]
    fn the_near_limit_warning_uses_an_attribute_that_survives() {
        assert_eq!(severity_code(85.0), "1;31");

        // Every code this can emit, listed so a future edit that reintroduces
        // blink fails here. Scanning for a "5" parameter would not do: the
        // orange code is `38;5;208`, where the 5 selects the 256-colour
        // palette and has nothing to do with blinking.
        let emitted: std::collections::BTreeSet<&str> = [0.0, 50.0, 65.0, 80.0, 100.0]
            .into_iter()
            .map(severity_code)
            .collect();
        assert_eq!(
            emitted,
            ["1;31", "32", "33", "38;5;208"].into_iter().collect(),
            "an unexpected SGR code appeared — check it survives Claude Code's \
             projection, which keeps only colour, dim, bold, italic, underline, \
             strikethrough and inverse"
        );
    }

    #[test]
    fn severity_steps_at_the_documented_thresholds() {
        assert_eq!(severity_code(49.9), "32");
        assert_eq!(severity_code(50.0), "33");
        assert_eq!(severity_code(64.9), "33");
        assert_eq!(severity_code(65.0), "38;5;208");
        assert_eq!(severity_code(79.9), "38;5;208");
        assert_eq!(severity_code(80.0), "1;31");
    }

    // The skull is gone on purpose: it was prepended only past the severe
    // threshold, so crossing it shifted the whole line — by two columns, or
    // more in the terminals that render the emoji double-width, and they
    // disagree. Colour carries severity now, at no cost in layout.
    #[test]
    fn no_marker_is_prepended_at_any_level() {
        for p in [0.0, 40.0, 80.0, 85.0, 100.0] {
            let seg = usage_parts(p, p).1;
            assert!(!seg.contains('💀'), "marker reappeared at {p}: {seg:?}");
            assert!(
                seg.starts_with('▓') || seg.starts_with('░'),
                "the bar must start the segment at {p}: {seg:?}"
            );
        }
    }

    #[test]
    fn the_reserve_shows_in_the_colour_not_in_the_digits() {
        unsafe {
            std::env::set_var("NO_COLOR", "1");
            std::env::remove_var("CLAUDE_CODE_AUTO_COMPACT_WINDOW");
        }
        // remaining == the 16.5% reserve: the *usable* window is fully spent,
        // but 83.5% of the real window is what has been used and what Claude
        // Code reports. The digits say so; severity is the colour's job, and
        // `severity_steps_at_the_documented_thresholds` covers that.
        let v = serde_json::json!({
            "context_window": { "remaining_percentage": 16.5, "context_window_size": 1_000_000 }
        });
        let seg = context_segment(&v).unwrap();
        assert!(seg.contains("84%"), "got {seg:?}");
        assert!(
            !seg.contains("100%"),
            "the old normalised figure is back: {seg:?}"
        );

        // A fresh context reads as nothing used, either way round.
        let v = serde_json::json!({
            "context_window": { "remaining_percentage": 100.0, "context_window_size": 1_000_000 }
        });
        assert!(context_segment(&v).unwrap().contains("0%"));

        // And a full one reads as full rather than overflowing.
        let v = serde_json::json!({
            "context_window": { "remaining_percentage": 0.0, "context_window_size": 1_000_000 }
        });
        assert!(context_segment(&v).unwrap().contains("100%"));
    }

    #[test]
    fn context_segment_absent_without_context_window() {
        let v = serde_json::json!({ "model": { "display_name": "Opus" } });
        assert!(context_segment(&v).is_none());
    }

    #[test]
    fn project_name_prefers_project_dir() {
        let v = serde_json::json!({
            "workspace": {"project_dir": "/a/b/myproj", "current_dir": "/a/b/myproj/sub"}
        });
        assert_eq!(project_name(&v).as_deref(), Some("myproj"));
    }

    #[test]
    fn project_name_falls_back_to_cwd() {
        let v = serde_json::json!({ "cwd": "/x/y/zproj" });
        assert_eq!(project_name(&v).as_deref(), Some("zproj"));
    }

    #[test]
    fn paint_respects_no_color() {
        unsafe { std::env::set_var("NO_COLOR", "1") };
        assert_eq!(paint("31", "hi"), "hi");
    }

    #[test]
    fn command_path_uses_forward_slashes() {
        // Whatever the platform, the rendered command must never contain a
        // backslash — Git Bash (used by Claude Code on Windows) would treat it
        // as an escape and the status line would render blank.
        let p = Path::new("base").join("bin").join("claude-acc");
        let rendered = command_path(&p);
        assert!(
            !rendered.contains('\\'),
            "command path must not contain backslashes: {rendered:?}"
        );
        assert!(rendered.contains("bin/claude-acc"), "got {rendered:?}");
    }
}
