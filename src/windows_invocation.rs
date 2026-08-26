// Hardened `claude` invocation for Windows.
//
// `claude` on Windows almost always resolves to an npm-installed `claude.cmd`
// shim, not a real .exe. Windows' CreateProcess has OS-level special-casing
// for launching a .cmd/.bat target: it implicitly re-runs the command line
// through cmd.exe, even when the caller only ever passed a structured argv
// (no shell involved on the caller's side). cmd.exe's own command-line
// parsing has quirks — most notably `%...%` percent-expansion and `&`/`|`/`^`
// metacharacters — that a plain argv-quoting scheme doesn't protect against.
// This is the exact same OS-level footgun behind Node's CVE-2024-27980
// (`child_process.spawn` invoking a .cmd/.bat file).
//
// Ported from github.com/stablyai/orca's `buildWindowsCommandInvocation` /
// `quoteCmdToken` (src/main/claude-accounts/windows-command-invocation.ts):
// rather than relying on whatever CreateProcess does implicitly, explicitly
// build the `cmd.exe /d /v:off /s /c "..."` invocation ourselves, with every
// token quoted so cmd.exe's own parser can't reinterpret it.
//
// The quoting logic below is only *called* from the `#[cfg(windows)]` half
// of `claude_command`, but is deliberately not itself gated on `windows` so
// it gets real unit-test coverage on every CI platform, not just the
// windows-latest runner. `allow(dead_code)` off Windows reflects that: it's
// unused in that build's production path, not actually dead.
#![cfg_attr(not(windows), allow(dead_code))]

use std::process::Command;

pub struct WindowsCommandInvocation {
    pub command: String,
    pub args: Vec<String>,
}

/// Quotes a single command-line token for safe embedding inside the
/// double-quoted string cmd.exe receives after `/c`. Rejects tokens
/// containing `"` or a line break outright — there's no way to embed those
/// safely in a `cmd.exe /c "..."` command line at all.
fn quote_cmd_token(value: &str) -> Result<String, String> {
    if value.contains('\r') || value.contains('\n') || value.contains('"') {
        return Err(format!(
            "Windows command tokens cannot contain quotes or line breaks: {value:?}"
        ));
    }
    // MSVCRT/CommandLineToArgvW rule: backslashes immediately before a
    // closing quote must be doubled, or they'd escape the quote instead of
    // terminating literally.
    let trailing_backslashes = value.chars().rev().take_while(|&c| c == '\\').count();
    let mut crt_escaped = value.to_string();
    crt_escaped.push_str(&"\\".repeat(trailing_backslashes));
    // cmd.exe still expands %VAR% inside a double-quoted string. Briefly
    // close the quote around each `%` and caret-escape it — caret escaping
    // only works outside quotes — then reopen the quote.
    let percent_escaped = crt_escaped.replace('%', "\"^%\"");
    Ok(format!("\"{percent_escaped}\""))
}

/// Builds a `cmd.exe /d /v:off /s /c "<command> <args...>"` invocation with
/// every token quoted against cmd.exe's own parsing. `command`/`args` are
/// the *real* argv you want to run (e.g. `"claude"`, `["auth", "login"]`) —
/// this function handles wrapping it for cmd.exe, not the other way round.
pub fn build_windows_command_invocation(
    command: &str,
    args: &[String],
) -> Result<WindowsCommandInvocation, String> {
    let comspec = std::env::var("ComSpec").unwrap_or_else(|_| "cmd.exe".to_string());
    let mut tokens = Vec::with_capacity(args.len() + 1);
    tokens.push(quote_cmd_token(command)?);
    for a in args {
        tokens.push(quote_cmd_token(a)?);
    }
    let command_line = tokens.join(" ");
    Ok(WindowsCommandInvocation {
        command: comspec,
        args: vec![
            "/d".to_string(),
            "/v:off".to_string(),
            "/s".to_string(),
            "/c".to_string(),
            format!("\"{command_line}\""),
        ],
    })
}

/// Builds a `Command` that runs `claude <args>`, hardened against the
/// Windows .cmd-shim quoting footgun on Windows and a plain
/// `Command::new("claude").args(args)` everywhere else. Callers still set
/// env vars on the returned `Command` as usual.
#[cfg(windows)]
pub fn claude_command(args: &[String]) -> Command {
    use std::os::windows::process::CommandExt;
    match build_windows_command_invocation("claude", args) {
        Ok(invocation) => {
            let mut cmd = Command::new(&invocation.command);
            for a in &invocation.args {
                // `raw_arg` appends the token exactly as given — no further
                // quoting from Rust — matching Node's `windowsVerbatimArguments:
                // true`. We already fully quoted each piece above; a second
                // layer of quoting would corrupt it.
                cmd.raw_arg(a);
            }
            cmd
        }
        Err(_) => {
            // A token had a quote/newline we can't safely embed in a cmd.exe
            // command line at all. Fall back to Rust's own argv-based
            // invocation rather than crash — still correct for a real .exe,
            // just not hardened against the .cmd-shim quirk for this call.
            let mut cmd = Command::new("claude");
            cmd.args(args);
            cmd
        }
    }
}

#[cfg(not(windows))]
pub fn claude_command(args: &[String]) -> Command {
    let mut cmd = Command::new("claude");
    cmd.args(args);
    cmd
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quote_cmd_token_wraps_plain_value_in_quotes() {
        assert_eq!(quote_cmd_token("work").unwrap(), "\"work\"");
    }

    #[test]
    fn quote_cmd_token_rejects_embedded_quote() {
        assert!(quote_cmd_token("a\"b").is_err());
    }

    #[test]
    fn quote_cmd_token_rejects_line_breaks() {
        assert!(quote_cmd_token("a\nb").is_err());
        assert!(quote_cmd_token("a\rb").is_err());
    }

    #[test]
    fn quote_cmd_token_doubles_trailing_backslashes() {
        // One trailing backslash must become two, or it would escape the
        // closing quote instead of terminating literally before it.
        assert_eq!(quote_cmd_token("C:\\path\\").unwrap(), "\"C:\\path\\\\\"");
    }

    #[test]
    fn quote_cmd_token_leaves_interior_backslashes_alone() {
        assert_eq!(
            quote_cmd_token("C:\\path\\to\\x").unwrap(),
            "\"C:\\path\\to\\x\""
        );
    }

    #[test]
    fn quote_cmd_token_escapes_percent_to_defeat_expansion() {
        assert_eq!(quote_cmd_token("50%").unwrap(), "\"50\"^%\"\"");
    }

    #[test]
    fn quote_cmd_token_handles_metacharacters_as_plain_text() {
        // These would be shell metacharacters to a naive cmd.exe invocation
        // (command chaining, piping, escaping) — quoting must neutralize
        // them, not merely pass them through.
        for value in ["a&b", "a|b", "a^b", "a<b", "a>b"] {
            let quoted = quote_cmd_token(value).unwrap();
            assert_eq!(quoted, format!("\"{value}\""));
        }
    }

    #[test]
    fn build_windows_command_invocation_quotes_every_token() {
        let invocation =
            build_windows_command_invocation("claude", &["auth".to_string(), "login".to_string()])
                .unwrap();
        assert_eq!(invocation.args[0], "/d");
        assert_eq!(invocation.args[1], "/v:off");
        assert_eq!(invocation.args[2], "/s");
        assert_eq!(invocation.args[3], "/c");
        assert_eq!(invocation.args[4], "\"\"claude\" \"auth\" \"login\"\"");
    }

    #[test]
    fn build_windows_command_invocation_rejects_unsafe_arg() {
        assert!(build_windows_command_invocation("claude", &["bad\"arg".to_string()]).is_err());
    }
}
