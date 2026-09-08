//! The browser bridge, and why it is not per-account.
//!
//! Claude Code pairs a session with the browser extension over a unix socket
//! in a directory whose name it builds from the OS user and nothing else:
//!
//! ```text
//! /tmp/claude-mcp-browser-bridge-<os user name>
//! ```
//!
//! The native host — launched by the browser, or by the desktop app — listens
//! on `<pid>.sock` in there; a session scans the directory and connects to
//! what it finds. `CLAUDE_CONFIG_DIR` is not part of the path, so every
//! account this tool creates shares one rendezvous point, and a session on one
//! account can be served by a native host paired with another. The second leg
//! of the same connection *is* per-account — it authenticates over
//! `bridge.claudeusercontent.com` as the OAuth identity of the config
//! directory in play — so the two halves can disagree, which surfaces as
//! "the OAuth token Claude Code is using belongs to a different claude.ai
//! account".
//!
//! Nothing here can change that: the path is computed inside the `claude`
//! binary from `os.userInfo().username`, with no environment input to hook.
//! What this module does is *see* it, so `doctor` can name who is listening
//! and under which config directory instead of leaving someone with an
//! unexplained "Browser extension is not connected".

use std::path::Path;
use std::process::Command;

/// A native host holding a socket in the bridge directory.
#[derive(Debug, PartialEq, Eq)]
pub struct Host {
    pub pid: u32,
    /// The host's own `CLAUDE_CONFIG_DIR`. `None` means it inherited none and
    /// is therefore running on the standard `~/.claude/` account — which is
    /// the usual case, since browsers and the desktop app are launched from
    /// the GUI rather than from a shell this tool has touched.
    pub config_dir: Option<String>,
    pub exe: String,
}

/// The rendezvous directory for `user`, exactly as the `claude` binary builds
/// it. Kept as a function rather than inlined so the shape is stated once and
/// tested — if upstream ever scopes it per config directory, this is the line
/// that changes.
pub fn bridge_dir(user: &str) -> String {
    format!("/tmp/claude-mcp-browser-bridge-{}", user)
}

/// `31672.sock` -> `31672`. Anything else in the directory is not ours.
pub fn socket_pid(file_name: &str) -> Option<u32> {
    file_name.strip_suffix(".sock")?.parse().ok()
}

/// Pids of every socket in `dir`, ascending. A missing or unreadable
/// directory is "nothing is listening", not an error: this is a diagnostic,
/// and a bridge that was never used has no directory at all.
pub fn listening_pids(dir: &Path) -> Vec<u32> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut pids: Vec<u32> = entries
        .flatten()
        .filter_map(|e| socket_pid(&e.file_name().to_string_lossy()))
        .collect();
    pids.sort_unstable();
    pids
}

/// One line of `ps -E -o pid=,command=`: the pid, then the command, then the
/// process environment appended after it.
///
/// The environment comes *after* the arguments, so when a token appears twice
/// the last one is the environment's — an argument that happens to look like
/// `CLAUDE_CONFIG_DIR=...` cannot shadow the real value. A config directory
/// containing a space would still be cut short; `ps` gives no way to tell that
/// apart, and this is a diagnostic rather than something decisions are made
/// on, so it is left as is.
pub fn parse_ps_line(line: &str) -> Option<Host> {
    let line = line.trim_start();
    let (pid, rest) = line.split_once(char::is_whitespace)?;
    let pid: u32 = pid.parse().ok()?;
    let rest = rest.trim_start();
    let exe = rest.split_whitespace().next()?.to_string();
    let config_dir = rest
        .split_whitespace()
        .filter_map(|t| t.strip_prefix("CLAUDE_CONFIG_DIR="))
        .next_back()
        .map(str::to_string);
    Some(Host {
        pid,
        config_dir,
        exe,
    })
}

/// Ask `ps` about `pids`, in one call, and return what it could tell us.
/// Pids that have since exited simply do not come back.
fn describe_pids(pids: &[u32]) -> Vec<Host> {
    if pids.is_empty() {
        return Vec::new();
    }
    let mut cmd = Command::new("ps");
    cmd.args(["-E", "-o", "pid=,command="]);
    for pid in pids {
        cmd.arg("-p").arg(pid.to_string());
    }
    let Ok(out) = cmd.output() else {
        return Vec::new();
    };
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(parse_ps_line)
        .collect()
}

/// Every native host currently holding a bridge socket for `user`.
///
/// Empty on Windows, where the bridge is a named pipe rather than a directory
/// of sockets — there is nothing to enumerate, so there is nothing to report.
pub fn hosts(user: &str) -> Vec<Host> {
    if cfg!(windows) {
        return Vec::new();
    }
    describe_pids(&listening_pids(Path::new(&bridge_dir(user))))
}

/// The OS user name the bridge directory is keyed by. `$USER` is the same
/// value `ps`/`whoami` would give for the process actually doing the pairing,
/// and getting it wrong only means reporting nothing.
pub fn os_user() -> Option<String> {
    std::env::var("USER").ok().filter(|u| !u.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bridge_directory_is_keyed_by_the_os_user_alone() {
        // Stated as a contract, not a formatting detail: the absence of the
        // config directory from this path is the whole reason the module
        // exists.
        assert_eq!(
            bridge_dir("petr"),
            "/tmp/claude-mcp-browser-bridge-petr",
            "upstream builds the name from os.userInfo().username only"
        );
    }

    #[test]
    fn only_sock_files_named_after_a_pid_count() {
        assert_eq!(socket_pid("31672.sock"), Some(31672));
        assert_eq!(socket_pid("31672"), None);
        assert_eq!(socket_pid("notes.sock"), None);
        assert_eq!(socket_pid(".sock"), None);
        assert_eq!(socket_pid("-1.sock"), None);
    }

    #[test]
    fn a_missing_bridge_directory_is_silence_not_an_error() {
        let missing = std::env::temp_dir().join(format!("cc-bridge-none-{}", std::process::id()));
        assert!(listening_pids(&missing).is_empty());
    }

    #[test]
    fn sockets_are_reported_in_a_stable_order() {
        let dir = std::env::temp_dir().join(format!("cc-bridge-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        for name in ["33985.sock", "31672.sock", "README", "x.sock"] {
            std::fs::write(dir.join(name), b"").unwrap();
        }
        assert_eq!(listening_pids(&dir), vec![31672, 33985]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    // Real `ps -E -o pid=,command=` output, trimmed: the desktop app's helper
    // running with no CLAUDE_CONFIG_DIR at all, and a CLI wrapper running on
    // an account.
    const PS: &str = concat!(
        "31672 /Applications/Claude.app/Contents/Helpers/chrome-native-host \
chrome-extension://fcoeoabgfenejglbffodgkkbkcdhcgfn/ USER=petr HOME=/Users/petr SHLVL=1\n",
        "  409 /Users/petr/.local/bin/claude --chrome-native-host USER=petr \
CLAUDE_CONFIG_DIR=/Users/petr/.claude-switch/accounts/work HOME=/Users/petr\n",
    );

    #[test]
    fn a_host_with_no_config_dir_is_the_standard_account() {
        let h = parse_ps_line(PS.lines().next().unwrap()).unwrap();
        assert_eq!(h.pid, 31672);
        assert_eq!(h.config_dir, None);
        assert_eq!(
            h.exe,
            "/Applications/Claude.app/Contents/Helpers/chrome-native-host"
        );
    }

    #[test]
    fn a_host_carrying_a_config_dir_is_attributed_to_it() {
        let h = parse_ps_line(PS.lines().nth(1).unwrap()).unwrap();
        assert_eq!(h.pid, 409);
        assert_eq!(
            h.config_dir.as_deref(),
            Some("/Users/petr/.claude-switch/accounts/work")
        );
        assert_eq!(h.exe, "/Users/petr/.local/bin/claude");
    }

    #[test]
    fn the_environment_wins_over_an_argument_that_looks_like_it() {
        // `ps` prints arguments first, environment second. Taking the first
        // match would let a process started with a lookalike argument
        // misreport which account is listening.
        let h = parse_ps_line(
            "42 /bin/claude --chrome-native-host CLAUDE_CONFIG_DIR=/decoy \
USER=petr CLAUDE_CONFIG_DIR=/real",
        )
        .unwrap();
        assert_eq!(h.config_dir.as_deref(), Some("/real"));
    }

    #[test]
    fn garbage_from_ps_is_dropped_rather_than_guessed_at() {
        assert!(parse_ps_line("").is_none());
        assert!(parse_ps_line("   ").is_none());
        assert!(parse_ps_line("notapid /bin/claude").is_none());
        assert!(parse_ps_line("42").is_none(), "a pid with no command");
    }
}
