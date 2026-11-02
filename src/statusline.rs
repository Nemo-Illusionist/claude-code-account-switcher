//! The `statusLine` block that points Claude Code at `claude-acc statusline`.
//! The CLI's `statusline --install` and the manager write the same one.

use std::path::Path;

use crate::config::binary_name;

/// The `statusLine` settings value for the CLI installed under `base_dir`.
pub fn entry(base_dir: &Path) -> serde_json::Value {
    let bin = base_dir.join("bin").join(binary_name());
    serde_json::json!({
        "type": "command",
        "command": format!("{} statusline", command_path(&bin)),
        "padding": 0,
    })
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

#[cfg(test)]
mod tests {
    use super::*;

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
