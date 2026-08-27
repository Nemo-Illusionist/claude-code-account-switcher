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

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct WindowsCommandInvocation {
    pub command: String,
    pub args: Vec<String>,
}

/// Why an invocation couldn't be built. The variants exist so the message
/// can be written in the user's language: the earlier `String` reason was
/// English no matter what, and showed up embedded in a Russian sentence.
#[derive(Debug, PartialEq)]
pub enum InvocationError {
    /// A token cmd.exe cannot carry at all. Holds the offending token.
    UnsupportedArg(String),
    /// `claude` is nowhere on PATH.
    NotFound,
}

/// Quotes a single command-line token for safe embedding inside the
/// double-quoted string cmd.exe receives after `/c`. Rejects tokens
/// containing `"` or a line break outright — there's no way to embed those
/// safely in a `cmd.exe /c "..."` command line at all.
fn quote_cmd_token(value: &str) -> Result<String, InvocationError> {
    if value.contains('\r') || value.contains('\n') || value.contains('"') {
        return Err(InvocationError::UnsupportedArg(value.to_string()));
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
) -> Result<WindowsCommandInvocation, InvocationError> {
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
///
/// `Err` carries a message fit to show a user: a token that cannot be
/// represented on a `cmd.exe` command line at all. Falling back to
/// `Command::new("claude")` there was the first design, and it is wrong for
/// the case this whole module exists for — that call finds no `.cmd` shim,
/// so the user got `program not found` instead of the reason.
#[cfg(windows)]
pub fn claude_command(args: &[String]) -> Result<Command, InvocationError> {
    use std::os::windows::process::CommandExt;
    // Resolved here rather than left to cmd.exe. Two reasons: a missing
    // `claude` becomes something we can say plainly, instead of cmd.exe
    // printing `'"claude"' is not recognized` and exiting 1 — indistinguishable
    // from claude itself failing; and handing cmd.exe an absolute path means
    // its search and ours can't pick different files.
    let claude = find_executable(
        "claude",
        std::env::current_dir().ok().as_deref(),
        std::env::var_os("PATH").as_deref(),
        std::env::var_os("PATHEXT").as_deref(),
    )
    .ok_or(InvocationError::NotFound)?;
    let invocation = build_windows_command_invocation(&claude.to_string_lossy(), args)?;
    let mut cmd = Command::new(&invocation.command);
    for a in &invocation.args {
        // `raw_arg` appends the token exactly as given — no further quoting
        // from Rust — matching Node's `windowsVerbatimArguments: true`. We
        // already fully quoted each piece above; a second layer of quoting
        // would corrupt it.
        cmd.raw_arg(a);
    }
    Ok(cmd)
}

#[cfg(not(windows))]
pub fn claude_command(args: &[String]) -> Result<Command, InvocationError> {
    let mut cmd = Command::new("claude");
    cmd.args(args);
    Ok(cmd)
}

/// Where `cmd.exe` would find `name`: the current directory first, then each
/// PATH entry, trying the bare name and then each PATHEXT extension.
///
/// Rust's own `Command::new` does none of this — it looks for a literal
/// `claude`/`claude.exe` and so never finds the `claude.cmd` an npm install
/// leaves, which is why `run`/`add`/`login` used to die with
/// `program not found` on the commonest Windows setup.
///
/// Pure, so the search order is testable on any platform.
pub fn find_executable(
    name: &str,
    cwd: Option<&Path>,
    path: Option<&OsStr>,
    pathext: Option<&OsStr>,
) -> Option<PathBuf> {
    let extensions: Vec<String> = pathext
        .map(|v| v.to_string_lossy().into_owned())
        .unwrap_or_else(|| ".COM;.EXE;.BAT;.CMD".to_string())
        .split(';')
        .filter(|e| !e.is_empty())
        .map(|e| e.to_string())
        .collect();

    let dirs = cwd
        .map(|d| d.to_path_buf())
        .into_iter()
        .chain(path.map(std::env::split_paths).into_iter().flatten());

    for dir in dirs {
        let base = dir.join(name);
        // A name that already carries its own extension is used as given —
        // that is what `cmd.exe` does too.
        if base.is_file() {
            return Some(base);
        }
        for ext in &extensions {
            let candidate = dir.join(format!("{}{}", name, ext));
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;
    use std::fs;

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("cc-winpath-{}-{}", tag, std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn os(v: &str) -> OsString {
        OsString::from(v)
    }

    // These tests pass PATHEXT in the same case as the files they create, so
    // they assert search *order* on any filesystem rather than accidentally
    // testing case sensitivity — Linux is case-sensitive and would fail on a
    // `.CMD` candidate for a `claude.cmd` file, while Windows and macOS would
    // not. Real Windows PATHEXT is uppercase, and matching a lowercase file
    // there works because the filesystem is case-insensitive; that is why the
    // search doesn't pay for a directory scan to normalise it.

    #[test]
    fn a_cmd_shim_is_found_where_rusts_own_lookup_finds_nothing() {
        // The whole bug: `Command::new("claude")` looks for a literal
        // `claude`/`claude.exe`, so an npm install's `claude.cmd` was
        // invisible and every run/add/login died with `program not found`.
        let dir = scratch("shim");
        fs::write(dir.join("claude.cmd"), "@echo off").unwrap();
        let found = find_executable(
            "claude",
            None,
            Some(&os(dir.to_str().unwrap())),
            Some(&os(".com;.exe;.bat;.cmd")),
        );
        assert_eq!(found, Some(dir.join("claude.cmd")));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn pathext_order_decides_between_two_candidates() {
        let dir = scratch("order");
        fs::write(dir.join("claude.cmd"), "").unwrap();
        fs::write(dir.join("claude.exe"), "").unwrap();
        let path = os(dir.to_str().unwrap());
        assert_eq!(
            find_executable("claude", None, Some(&path), Some(&os(".exe;.cmd"))),
            Some(dir.join("claude.exe"))
        );
        assert_eq!(
            find_executable("claude", None, Some(&path), Some(&os(".cmd;.exe"))),
            Some(dir.join("claude.cmd"))
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_current_directory_is_searched_before_path() {
        // cmd.exe looks there first, so a lookup that didn't would pick a
        // different file than the shell would have.
        let base = scratch("cwd");
        let here = base.join("here");
        let elsewhere = base.join("elsewhere");
        fs::create_dir_all(&here).unwrap();
        fs::create_dir_all(&elsewhere).unwrap();
        fs::write(here.join("claude.cmd"), "").unwrap();
        fs::write(elsewhere.join("claude.cmd"), "").unwrap();
        assert_eq!(
            find_executable(
                "claude",
                Some(&here),
                Some(&os(elsewhere.to_str().unwrap())),
                Some(&os(".cmd"))
            ),
            Some(here.join("claude.cmd"))
        );
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn path_entries_are_tried_in_order() {
        let base = scratch("path-order");
        let first = base.join("first");
        let second = base.join("second");
        fs::create_dir_all(&first).unwrap();
        fs::create_dir_all(&second).unwrap();
        fs::write(second.join("claude.cmd"), "").unwrap();
        let path = std::env::join_paths([&first, &second]).unwrap();
        assert_eq!(
            find_executable("claude", None, Some(&path), Some(&os(".cmd"))),
            Some(second.join("claude.cmd")),
            "an empty earlier entry must not stop the search"
        );
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn a_name_that_carries_its_own_extension_is_used_as_given() {
        let dir = scratch("explicit");
        fs::write(dir.join("claude.cmd"), "").unwrap();
        assert_eq!(
            find_executable(
                "claude.cmd",
                None,
                Some(&os(dir.to_str().unwrap())),
                Some(&os(".EXE"))
            ),
            Some(dir.join("claude.cmd"))
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn nothing_anywhere_is_none_rather_than_a_guess() {
        let dir = scratch("absent");
        assert_eq!(
            find_executable(
                "claude",
                None,
                Some(&os(dir.to_str().unwrap())),
                Some(&os(".cmd"))
            ),
            None
        );
        assert_eq!(find_executable("claude", None, None, None), None);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_directory_named_like_the_binary_is_not_mistaken_for_it() {
        let dir = scratch("dir-trap");
        fs::create_dir_all(dir.join("claude.cmd")).unwrap();
        assert_eq!(
            find_executable(
                "claude",
                None,
                Some(&os(dir.to_str().unwrap())),
                Some(&os(".cmd"))
            ),
            None
        );
        let _ = fs::remove_dir_all(&dir);
    }

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
