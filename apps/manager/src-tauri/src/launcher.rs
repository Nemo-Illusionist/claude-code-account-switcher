//! Hand an account to the bundled CLI in a new Terminal window.
//!
//! Terminal runs a `.command` file in a fresh login shell with its own
//! environment, so nothing of this process is inherited. The launcher below
//! is constant code; the account, directory, `HOME` and CLI path reach it as
//! lines of a private request file and are only ever expanded as quoted
//! arguments. `open` succeeding means Launch Services took the file, nothing
//! more — the launcher deleting its request directory is the signal that it
//! actually ran.
use crate::backend::AppError;
// Only the macOS launcher has a prompt to word.
#[cfg(target_os = "macos")]
use crate::copy;
use claude_acc::gui::LaunchAction;
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
// Only the macOS launcher reports a state; elsewhere there is no launch.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub enum LaunchState {
    /// The launcher ran in Terminal and handed over to the CLI. Whatever the
    /// CLI or Claude Code says next is in that window.
    Started,
    /// Terminal accepted the request but had not run it yet when we stopped
    /// waiting — a first-launch permission prompt can do this.
    Pending,
}

#[derive(Debug, Clone, Serialize)]
pub struct Launch {
    pub state: LaunchState,
}

#[derive(Debug, Clone, Serialize)]
pub struct Installation {
    /// The bundled CLI's version, as it reports it; "unknown" if it would not.
    pub bundled_version: String,
    pub bundled_available: bool,
    /// `claude-acc --version` of a separately installed CLI, if any.
    pub installed_version: Option<String>,
}

const CLI_NAME: &str = if cfg!(windows) {
    "claude-acc.exe"
} else {
    "claude-acc"
};

/// The CLI bundled beside this executable — never one found on `PATH`.
fn sidecar() -> Result<PathBuf, AppError> {
    let exe = std::env::current_exe().map_err(|_| AppError::CliMissing)?;
    let path = exe.parent().ok_or(AppError::CliMissing)?.join(CLI_NAME);
    path.is_file().then_some(path).ok_or(AppError::CliMissing)
}

/// `<cli> --version`, trimmed, or `None` if it is missing or will not say.
fn version_of(cli: &Path) -> Option<String> {
    let out = std::process::Command::new(cli)
        .arg("--version")
        .output()
        .ok()
        .filter(|o| o.status.success())?;
    Some(String::from_utf8(out.stdout).ok()?.trim().to_string())
}

/// Diagnostics only: runs both CLIs, so callers cache the answer.
pub fn installation(base_dir: &Path) -> Installation {
    let installed = base_dir.join("bin").join(CLI_NAME);
    let bundled = sidecar().ok();
    Installation {
        bundled_version: bundled
            .as_deref()
            .and_then(version_of)
            .map(|v| v.trim_start_matches("claude-acc ").to_string())
            .unwrap_or_else(|| "unknown".into()),
        bundled_available: bundled.is_some(),
        installed_version: installed
            .is_file()
            .then(|| version_of(&installed))
            .flatten(),
    }
}

#[cfg(target_os = "macos")]
pub fn open_terminal(
    home: &Path,
    directory: &Path,
    account: &str,
    action: LaunchAction,
) -> Result<Launch, AppError> {
    let (exited, press_return) = (
        copy::text(copy::Text::LauncherExited, copy::LANG),
        copy::text(copy::Text::LauncherPressReturn, copy::LANG),
    );
    let request = Request {
        home,
        directory,
        cli: &sidecar()?,
        account,
        action,
        exited: &exited,
        press_return: &press_return,
    };
    macos::launch(&request, "Terminal", std::time::Duration::from_secs(10))
}

#[cfg(not(target_os = "macos"))]
pub fn open_terminal(
    _home: &Path,
    _directory: &Path,
    _account: &str,
    _action: LaunchAction,
) -> Result<Launch, AppError> {
    Err(AppError::TerminalMacosOnly)
}

#[cfg(target_os = "macos")]
struct Request<'a> {
    home: &'a Path,
    directory: &'a Path,
    cli: &'a Path,
    account: &'a str,
    action: LaunchAction,
    /// The prompt shown if the CLI fails, already in the user's language.
    /// Data like every other line: printed with `%s`, never interpreted.
    exited: &'a str,
    press_return: &'a str,
}

#[cfg(target_os = "macos")]
mod macos {
    use super::{AppError, Launch, LaunchState, Request};
    use std::fs;
    use std::io::Write;
    use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
    use std::path::Path;
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

    /// Constant code. `HOME` is exported so the CLI resolves accounts under
    /// the same home this app showed them from; the shared credential-store
    /// override is dropped because it would point every account at one store.
    pub(super) const LAUNCHER: &str = r#"#!/bin/sh
set -u
request=$(dirname "$0")
{
  IFS= read -r home
  IFS= read -r directory
  IFS= read -r cli
  IFS= read -r action
  IFS= read -r seed
  IFS= read -r account
  IFS= read -r exited
  IFS= read -r press_return
} < "$request/request" || exit 1
# Consumed: the app waits for this directory to disappear.
rm -rf -- "$request"
export HOME="$home"
unset CLAUDE_SECURESTORAGE_CONFIG_DIR
cd -- "$directory" || exit 1
# A fixed word from the app, never user data, so never split from a value.
if [ "$seed" = seed ]; then set -- --seed; else set --; fi
"$cli" "$action" "$@" "$account"
status=$?
if [ "$status" -ne 0 ]; then
  printf '\n%s %s\n%s\n' "$exited" "$status" "$press_return"
  read -r _
fi
exit "$status"
"#;

    /// The request file body: one value per line, in the order `LAUNCHER`
    /// reads them. A value containing a line break could shift the others
    /// into the wrong variable, so any control character is refused.
    pub(super) fn request_body(r: &Request) -> Result<String, AppError> {
        let mut body = String::new();
        for value in [
            r.home.to_str(),
            r.directory.to_str(),
            r.cli.to_str(),
            Some(r.action.command()),
            Some(if r.action.seeds() { "seed" } else { "no-seed" }),
            Some(r.account),
            Some(r.exited),
            Some(r.press_return),
        ] {
            let value = value.ok_or(claude_acc::gui::GuiError::InvalidDirectory)?;
            if value.is_empty() || value.chars().any(char::is_control) {
                return Err(claude_acc::gui::GuiError::InvalidDirectory.into());
            }
            body.push_str(value);
            body.push('\n');
        }
        Ok(body)
    }

    /// Write the request, hand the launcher to `terminal`, and wait up to
    /// `wait` for it to run. On failure before the hand-off the request is
    /// removed; after it, the launcher removes it itself.
    pub(super) fn launch(r: &Request, terminal: &str, wait: Duration) -> Result<Launch, AppError> {
        let body = request_body(r)?;
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("claude-acc-launch-{}-{nanos}", std::process::id()));
        // Private from the first instant; `create` also refuses an existing
        // path, so a planted directory or symlink is never written through.
        fs::DirBuilder::new()
            .mode(0o700)
            .create(&root)
            .map_err(|_| AppError::LauncherFailed)?;
        let opened = write(&root, &body).and_then(|script| {
            let status = std::process::Command::new("/usr/bin/open")
                .args(["-a", terminal])
                .arg(script)
                .status()
                .map_err(|_| AppError::TerminalFailed)?;
            status
                .success()
                .then_some(())
                .ok_or(AppError::TerminalFailed)
        });
        if let Err(e) = opened {
            let _ = fs::remove_dir_all(&root);
            return Err(e);
        }
        Ok(Launch {
            state: wait_consumed(&root, wait),
        })
    }

    fn write(root: &Path, body: &str) -> Result<std::path::PathBuf, AppError> {
        let file = |name: &str, mode: u32, content: &str| {
            fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(mode)
                .open(root.join(name))
                .and_then(|mut f| f.write_all(content.as_bytes()))
                .map_err(|_| AppError::LauncherFailed)
        };
        file("request", 0o600, body)?;
        file("Launch.command", 0o700, LAUNCHER)?;
        Ok(root.join("Launch.command"))
    }

    pub(super) fn wait_consumed(root: &Path, wait: Duration) -> LaunchState {
        let deadline = Instant::now() + wait;
        while root.exists() {
            if Instant::now() >= deadline {
                return LaunchState::Pending;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        LaunchState::Started
    }
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::macos::{LAUNCHER, request_body};
    use super::*;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::time::Duration;

    /// A stand-in CLI that records what it was given and never authenticates.
    fn fake_cli(dir: &Path) -> PathBuf {
        let cli = dir.join("fake cli");
        fs::write(
            &cli,
            "#!/bin/sh\nout=\"$(dirname \"$0\")/seen\"\n\
             { printf 'arg=%s\\n' \"$@\"; printf 'home=%s\\n' \"$HOME\"; \
             printf 'pwd=%s\\n' \"$(pwd -P)\"; \
             printf 'store=%s\\n' \"${CLAUDE_SECURESTORAGE_CONFIG_DIR-unset}\"; } > \"$out.tmp\"\n\
             mv \"$out.tmp\" \"$out\"\n",
        )
        .unwrap();
        fs::set_permissions(&cli, fs::Permissions::from_mode(0o700)).unwrap();
        cli
    }

    fn scratch(tag: &str) -> PathBuf {
        let root = std::env::temp_dir()
            .canonicalize()
            .unwrap()
            .join(format!("cc-launch-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("O'Brien $(touch INJECTED) dir")).unwrap();
        root
    }

    fn expected(root: &Path, project: &Path, account: &str) -> String {
        format!(
            "arg=run\narg={account}\nhome={}\npwd={}\nstore=unset\n",
            root.join("home").display(),
            project.display()
        )
    }

    #[test]
    fn launcher_treats_every_request_value_as_data() {
        let root = scratch("data");
        let project = root.join("O'Brien $(touch INJECTED) dir");
        let cli = fake_cli(&root);
        let account = "work'; touch INJECTED; #";
        let request = root.join("request-dir");
        fs::create_dir(&request).unwrap();
        let body = request_body(&Request {
            home: &root.join("home"),
            directory: &project,
            cli: &cli,
            account,
            action: LaunchAction::Run,
            exited: "exited with status",
            press_return: "Press Return.",
        })
        .unwrap();
        fs::write(request.join("request"), body).unwrap();
        fs::write(request.join("Launch.command"), LAUNCHER).unwrap();
        let status = std::process::Command::new("/bin/sh")
            .arg(request.join("Launch.command"))
            .env("CLAUDE_SECURESTORAGE_CONFIG_DIR", "/shared")
            .env("HOME", "/somewhere/else")
            .current_dir("/")
            .status()
            .unwrap();
        assert!(status.success());
        assert_eq!(
            fs::read_to_string(root.join("seen")).unwrap(),
            expected(&root, &project, account)
        );
        assert!(!request.exists(), "the launcher consumes its request");
        assert!(!root.join("INJECTED").exists());
        assert!(!project.join("INJECTED").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn only_a_seeded_add_passes_seed_to_the_cli() {
        let root = scratch("seed");
        let cli = fake_cli(&root);
        for (action, account, args) in [
            (
                LaunchAction::AddSeeded,
                "work",
                "arg=add\narg=--seed\narg=work\n",
            ),
            (LaunchAction::Add, "work", "arg=add\narg=work\n"),
        ] {
            let request = root.join("request-dir");
            fs::create_dir(&request).unwrap();
            let body = request_body(&Request {
                home: &root.join("home"),
                directory: &root,
                cli: &cli,
                account,
                action,
                exited: "x",
                press_return: "y",
            })
            .unwrap();
            fs::write(request.join("request"), body).unwrap();
            fs::write(request.join("Launch.command"), LAUNCHER).unwrap();
            let status = std::process::Command::new("/bin/sh")
                .arg(request.join("Launch.command"))
                .status()
                .unwrap();
            assert!(status.success());
            let seen = fs::read_to_string(root.join("seen")).unwrap();
            assert!(seen.starts_with(args), "{action:?} {account}: {seen}");
        }
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_failing_cli_gets_the_prompt_from_data_printed_literally() {
        // The prompt is translated copy, so it is a request line like any
        // other: printed through `%s`, never parsed as a format or as shell.
        let root = scratch("prompt");
        let cli = root.join("failing cli");
        fs::write(&cli, "#!/bin/sh\nexit 3\n").unwrap();
        fs::set_permissions(&cli, fs::Permissions::from_mode(0o700)).unwrap();
        let request = root.join("request-dir");
        fs::create_dir(&request).unwrap();
        let exited = "100%s $(touch INJECTED) exited with";
        let body = request_body(&Request {
            home: &root.join("home"),
            directory: &root,
            cli: &cli,
            account: "work",
            action: LaunchAction::Run,
            exited,
            press_return: "Press Return.",
        })
        .unwrap();
        fs::write(request.join("request"), body).unwrap();
        fs::write(request.join("Launch.command"), LAUNCHER).unwrap();
        let out = std::process::Command::new("/bin/sh")
            .arg(request.join("Launch.command"))
            .stdin(std::process::Stdio::null())
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(3), "the CLI's status is passed on");
        assert_eq!(
            String::from_utf8(out.stdout).unwrap(),
            format!("\n{exited} 3\nPress Return.\n")
        );
        assert!(!root.join("INJECTED").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_value_that_could_shift_the_request_lines_is_refused() {
        let root = Path::new("/tmp");
        for (directory, account) in [("/tmp/a\nb", "work"), ("/tmp", "work\nrun"), ("/tmp", "")] {
            let r = Request {
                home: root,
                directory: Path::new(directory),
                cli: root,
                account,
                action: LaunchAction::Login,
                exited: "x",
                press_return: "y",
            };
            assert!(request_body(&r).is_err(), "{directory:?} {account:?}");
        }
    }

    #[test]
    fn a_request_nobody_runs_is_reported_pending_not_started() {
        let root = scratch("pending");
        assert_eq!(
            macos::wait_consumed(&root, Duration::from_millis(200)),
            LaunchState::Pending
        );
        fs::remove_dir_all(&root).unwrap();
        assert_eq!(
            macos::wait_consumed(&root, Duration::from_millis(200)),
            LaunchState::Started
        );
    }

    #[test]
    fn launch_payloads_match_the_frontend_contract_sample() {
        let sample: serde_json::Value =
            serde_json::from_str(include_str!("../../src/contract.sample.json")).unwrap();
        let launch = Launch {
            state: LaunchState::Started,
        };
        assert_eq!(sample["launch"], serde_json::to_value(launch).unwrap());
        let installation = Installation {
            bundled_version: "0.21.0".into(),
            bundled_available: true,
            installed_version: None,
        };
        assert_eq!(
            sample["installation"],
            serde_json::to_value(installation).unwrap()
        );
        let errors = [
            AppError::CliMissing,
            AppError::LauncherFailed,
            AppError::TerminalFailed,
            AppError::TerminalMacosOnly,
            AppError::BackendFailed,
        ];
        assert_eq!(
            sample["manager_errors"],
            serde_json::to_value(errors).unwrap()
        );
        assert_eq!(
            serde_json::to_value(AppError::from(claude_acc::gui::GuiError::IdentityDrift)).unwrap(),
            "identity_drift"
        );
    }

    /// Opens a real Terminal window running a stand-in CLI: no Claude Code,
    /// no login, no real account. Manual, because it needs a GUI session:
    /// `cargo test --release -- --ignored real_terminal`.
    #[test]
    #[ignore]
    fn real_terminal_runs_the_cli_with_this_apps_home_and_directory() {
        let root = scratch("terminal");
        let project = root.join("O'Brien $(touch INJECTED) dir");
        let cli = fake_cli(&root);
        let r = Request {
            home: &root.join("home"),
            directory: &project,
            cli: &cli,
            account: "work",
            action: LaunchAction::Run,
            exited: "exited with status",
            press_return: "Press Return.",
        };
        let launch = macos::launch(&r, "Terminal", Duration::from_secs(30)).unwrap();
        assert_eq!(launch.state, LaunchState::Started);
        let seen = root.join("seen");
        for _ in 0..100 {
            if seen.exists() {
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        assert_eq!(
            fs::read_to_string(&seen).unwrap(),
            expected(&root, &project, "work")
        );
        assert!(!project.join("INJECTED").exists());
        fs::remove_dir_all(root).unwrap();
    }
}
