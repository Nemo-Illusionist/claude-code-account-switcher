use std::io::Read;
use std::path::Path;
use std::process::Command;

use serde_json::Value;

use crate::config::AppConfig;
use crate::i18n::{I18n, Msg};
use crate::{resolve, statusline};

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

/// Default share of the context window Claude Code reserves for auto-compaction.
const AUTO_COMPACT_BUFFER_PCT: f64 = 16.5;

/// Context-window usage segment from Claude Code's `context_window` block.
///
/// The number shown is the plain used percentage of the whole window, so it
/// matches what Claude Code reports for the same session — 393.5k of 1M reads
/// as 39% in both places.
///
/// It did not used to. The percentage was scaled to the *usable* window, with
/// Claude Code's auto-compaction reserve removed, which pushed the same
/// session to 47% and left two numbers for one quantity with no way to tell
/// which was which. Agreeing with the application wins: a status line exists
/// to be glanced at, and a figure that disagrees with the app is worse than
/// one that needs a moment's thought about headroom.
///
/// The reserve still decides the *colour*, which is where it carries real
/// meaning — red keeps saying "compaction is about to kick in" rather than an
/// arbitrary 80% of the raw window. So the digits answer "how full is this
/// conversation" and the colour answers "how close is the wall".
fn context_segment(v: &Value) -> Option<String> {
    let remaining = v
        .pointer("/context_window/remaining_percentage")
        .and_then(Value::as_f64)?;
    // `context_window_size`, not `total_tokens` — the latter has never been a
    // field Claude Code sends, so this read always missed and silently fell
    // back to the default below. Harmless until someone sets
    // CLAUDE_CODE_AUTO_COMPACT_WINDOW, which is a *token count*: dividing it
    // by a window five times too large made the reserve look five times too
    // small, and the meter under-reported on every 200k session.
    let total = v
        .pointer("/context_window/context_window_size")
        .and_then(Value::as_f64)
        .filter(|t| *t > 0.0)
        .unwrap_or(1_000_000.0);

    let acw = std::env::var("CLAUDE_CODE_AUTO_COMPACT_WINDOW")
        .ok()
        .and_then(|s| s.trim().parse::<f64>().ok());
    Some(usage_segment(
        (100.0 - remaining).clamp(0.0, 100.0),
        used_of_usable(remaining, total, acw),
    ))
}

/// How much of the *usable* window is spent, 0–100.
///
/// `acw` is `CLAUDE_CODE_AUTO_COMPACT_WINDOW` as a token count, already read
/// from the environment — passed in rather than read here so this can be
/// asserted without touching process state the other tests in this file also
/// mutate.
fn used_of_usable(remaining: f64, total: f64, acw: Option<f64>) -> f64 {
    let buffer_pct = acw
        .filter(|a| *a > 0.0)
        .map(|a| (a / total * 100.0).min(100.0))
        .unwrap_or(AUTO_COMPACT_BUFFER_PCT);

    let usable_remaining = (((remaining - buffer_pct) / (100.0 - buffer_pct)) * 100.0).max(0.0);
    (100.0 - usable_remaining).clamp(0.0, 100.0)
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

    let mut root = std::fs::read_to_string(&settings_path)
        .ok()
        .and_then(|s| serde_json::from_str::<Value>(&s).ok())
        .filter(Value::is_object)
        .unwrap_or_else(|| serde_json::json!({}));

    root["statusLine"] = statusline::entry(&config.base_dir);

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

    // The reserve is a token count, so it only converts to a percentage
    // correctly against the real window size. We read `total_tokens`, which
    // Claude Code has never sent — the read always missed and silently used
    // the 1M default, making the reserve five times too small on a 200k
    // session and the meter under-report.
    #[test]
    fn the_compact_reserve_is_measured_against_the_real_window_size() {
        // 40k of a 200k window is a 20% reserve, and exactly that much
        // remains — so the usable window is spent.
        assert_eq!(used_of_usable(20.0, 200_000.0, Some(40_000.0)), 100.0);

        // Against the 1M the old code fell back to, the same reserve reads as
        // 4% and the window looks 5/6 spent. This is the bug, pinned.
        let wrong = used_of_usable(20.0, 1_000_000.0, Some(40_000.0));
        assert!((wrong - 83.33).abs() < 0.1, "got {wrong}");
    }

    #[test]
    fn with_no_reserve_set_the_default_share_applies() {
        // Remaining == the default 16.5% reserve: usable window fully spent.
        assert_eq!(used_of_usable(16.5, 1_000_000.0, None), 100.0);
        // An empty context reads as nothing used.
        assert_eq!(used_of_usable(100.0, 1_000_000.0, None), 0.0);
        // A zero or negative reserve is ignored rather than dividing by it.
        assert_eq!(used_of_usable(16.5, 1_000_000.0, Some(0.0)), 100.0);
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
}
