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
// context window, and — for Pro/Max — the live `rate_limits`), and renders
// whatever we print, ANSI colors included. We add the one thing Claude Code
// can't know: which managed account this session is running under
// (from CLAUDE_CONFIG_DIR).
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
    if let Some(pct) = v
        .pointer("/rate_limits/five_hour/used_percentage")
        .and_then(Value::as_f64)
    {
        segs.push(usage_segment(pct));
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

/// A colored 10-cell bar + percentage for a 0–100 value, green/yellow/red by
/// how close to the limit it is.
fn usage_segment(pct: f64) -> String {
    let p = pct.clamp(0.0, 100.0);
    let code = if p >= 90.0 {
        "31"
    } else if p >= 70.0 {
        "33"
    } else {
        "32"
    };
    let filled = ((p / 100.0) * BAR_WIDTH as f64).round() as usize;
    let filled = filled.min(BAR_WIDTH);
    let bar = format!("{}{}", "▓".repeat(filled), "░".repeat(BAR_WIDTH - filled));
    format!("{} {}%", paint(code, &bar), p.round() as i64)
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
    fn usage_segment_includes_rounded_percent() {
        // NO_COLOR keeps the assertion free of escape codes.
        unsafe { std::env::set_var("NO_COLOR", "1") };
        let seg = usage_segment(32.4);
        assert!(seg.contains("32%"), "got {seg:?}");
        assert!(seg.contains('▓') && seg.contains('░'), "got {seg:?}");
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
