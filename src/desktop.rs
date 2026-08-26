// Claude Desktop profiles.
//
// The desktop app is Electron, so it honours Chromium's `--user-data-dir`
// switch. Pointing it at a directory of our own gives a fully isolated app
// profile — the same move this tool already makes for the CLI with
// `CLAUDE_CONFIG_DIR`, and unlike the CLI accounts these can run at the same
// time: the app takes no single-instance lock, and Chromium's own lock lives
// inside each user-data-dir.
//
// Nothing is copied and nothing is killed, so none of the "swap the signed-in
// state on disk" problems apply — see issue #75 for the research.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::config::AppConfig;

/// Overrides the located `Claude.app`. For testing, and for anyone whose app
/// lives somewhere the search below doesn't reach.
pub const APP_ENV: &str = "CLAUDE_ACC_DESKTOP_APP";

/// `~/.claude-switch/desktop/` — one subdirectory per profile, alongside
/// `accounts/`.
pub fn profiles_dir(config: &AppConfig) -> PathBuf {
    config.base_dir.join("desktop")
}

pub fn profile_path(config: &AppConfig, name: &str) -> PathBuf {
    profiles_dir(config).join(name)
}

pub fn profile_exists(config: &AppConfig, name: &str) -> bool {
    profile_path(config, name).is_dir()
}

pub fn list_profiles(config: &AppConfig) -> io::Result<Vec<String>> {
    let dir = profiles_dir(config);
    if !dir.exists() {
        return Ok(vec![]);
    }
    let mut names = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir()
            && let Some(name) = entry.file_name().to_str()
        {
            names.push(name.to_string());
        }
    }
    names.sort();
    Ok(names)
}

/// The app's own, unmanaged profile — the one it uses when launched normally.
/// Shown for orientation; never written to.
///
/// Electron's `userData` default, per platform. On Windows this is the plain
/// install's location; a Store install keeps its own copy inside the package
/// container (see `packaged_install`), which we never touch.
pub fn standard_profile() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        dirs::home_dir().map(|h| h.join("Library/Application Support/Claude"))
    }
    #[cfg(windows)]
    {
        // %APPDATA%\Claude
        dirs::data_dir().map(|d| d.join("Claude"))
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        // ~/.config/Claude — what the community Linux packages use.
        dirs::config_dir().map(|d| d.join("Claude"))
    }
}

/// The abbreviated path `list` uses for the app's own profile.
pub fn standard_label() -> &'static str {
    if cfg!(target_os = "macos") {
        "~/Library/…/Claude/"
    } else if cfg!(windows) {
        "%APPDATA%\\Claude\\"
    } else {
        "~/.config/Claude/"
    }
}

/// Whether the Store (MSIX) build is installed: `%LOCALAPPDATA%\Packages\`
/// holds a `Claude_*` directory.
///
/// It matters because that build cannot be driven this way. Its executable
/// sits under `WindowsApps`, is activated through `shell:AppsFolder` rather
/// than run directly — which is no way to pass a command-line switch — and
/// the package container redirects file paths, so even a switch that arrived
/// would not point where it says. Refusing is the only safe answer: the
/// failure mode of guessing is opening the real profile while claiming to
/// have opened another one.
pub fn packaged_install() -> bool {
    #[cfg(windows)]
    {
        dirs::data_local_dir().is_some_and(|d| has_claude_package(&d.join("Packages")))
    }
    #[cfg(not(windows))]
    {
        false
    }
}

/// The newest `app-<version>` directory under a Squirrel install root, by
/// name — Squirrel's own ordering, and the versions sort correctly as strings
/// only within a major, so the comparison is on the whole name and ties go to
/// the later entry. Split out to be testable off Windows.
#[cfg_attr(not(windows), allow(dead_code))] // Windows-only, tested everywhere
fn newest_squirrel_app(root: &Path) -> Option<PathBuf> {
    let mut apps: Vec<PathBuf> = fs::read_dir(root)
        .ok()?
        .filter_map(Result::ok)
        .filter(|e| {
            e.file_name()
                .to_str()
                .is_some_and(|n| n.starts_with("app-"))
        })
        .map(|e| e.path())
        .collect();
    apps.sort();
    apps.pop()
}

/// The first `name` on `PATH`. `Command` would resolve it too, but knowing
/// the absolute path up front is what lets `find_app` report "not installed"
/// instead of failing at launch.
#[cfg_attr(any(target_os = "macos", windows), allow(dead_code))] // Linux-only
fn on_path(name: &str) -> Option<PathBuf> {
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths)
            .map(|dir| dir.join(name))
            .find(|candidate| candidate.is_file())
    })
}

/// A `Claude_*` entry directly under `packages_root`. Split out so the
/// matching is testable off Windows.
#[cfg_attr(not(windows), allow(dead_code))] // Windows-only, tested everywhere
fn has_claude_package(packages_root: &Path) -> bool {
    let Ok(entries) = fs::read_dir(packages_root) else {
        return false;
    };
    entries.filter_map(Result::ok).any(|e| {
        e.file_name()
            .to_str()
            .is_some_and(|n| n.starts_with("Claude_"))
    })
}

/// A running Claude Desktop process, and the profile it was launched on
/// (`None` for the app's own).
#[derive(Debug, PartialEq)]
pub struct Instance {
    pub pid: u32,
    pub profile: Option<String>,
}

/// Every Claude Desktop instance currently running.
///
/// Signing in has to happen with none of them open: it finishes through a
/// `claude://` link, and the system hands that to whichever instance is
/// registered for the scheme — not necessarily the one that started the
/// login. Once a profile is signed in, instances coexist happily.
pub fn running_instances() -> Vec<Instance> {
    if !cfg!(target_os = "macos") {
        return Vec::new();
    }
    let Ok(out) = Command::new("ps")
        .args(["-ax", "-o", "pid=,command="])
        .output()
    else {
        return Vec::new();
    };
    parse_ps(&String::from_utf8_lossy(&out.stdout))
}

/// Main app processes from `ps` output, with the profile each is on.
///
/// The **first token** has to be the app's executable. Matching the line
/// anywhere would count any process that merely mentions the path in its
/// arguments — a shell running a script about it, this tool being told where
/// the app is.
///
/// That alone excludes today's helpers, whose bundles sit at
/// `.../Frameworks/Claude Helper.app/...` and `.../Claude Helper
/// (Renderer).app/...`, so their first token stops at the space before
/// `Helper`. The `--type=` check is a second net for a future helper whose
/// path happens to have no space in it; every Chromium child carries one.
fn parse_ps(output: &str) -> Vec<Instance> {
    output
        .lines()
        .filter_map(|line| {
            let (pid, rest) = line.trim_start().split_once(char::is_whitespace)?;
            let executable = rest.split_whitespace().next()?;
            if !executable.ends_with("/Contents/MacOS/Claude") || rest.contains("--type=") {
                return None;
            }
            Some(Instance {
                pid: pid.parse().ok()?,
                profile: rest
                    .split_whitespace()
                    .find_map(|arg| arg.strip_prefix("--user-data-dir="))
                    .map(|dir| {
                        Path::new(dir)
                            .file_name()
                            .map(|n| n.to_string_lossy().into_owned())
                            .unwrap_or_else(|| dir.to_string())
                    }),
            })
        })
        .collect()
}

/// Whether reading a profile's identity works here. The credential scheme is
/// Chromium's, but the way the key is stored is not: macOS uses the Keychain,
/// Windows DPAPI, Linux libsecret or a plaintext fallback. Only the macOS
/// half is implemented — see #75.
pub fn identity_supported() -> bool {
    cfg!(target_os = "macos")
}

/// The installed `Claude.app`, or `None` if it isn't where we look.
pub fn find_app() -> Option<PathBuf> {
    pick_app(std::env::var(APP_ENV).ok().as_deref(), &app_candidates())
}

/// An override, if it points at something real, otherwise the first candidate
/// that exists. A set-but-wrong override resolves to nothing rather than
/// falling back, so a typo is visible instead of silently launching the app
/// the override meant to replace.
fn pick_app(override_path: Option<&str>, candidates: &[PathBuf]) -> Option<PathBuf> {
    if let Some(custom) = override_path.filter(|v| !v.is_empty()) {
        let path = PathBuf::from(custom);
        return path.exists().then_some(path);
    }
    candidates.iter().find(|p| p.exists()).cloned()
}

fn app_candidates() -> Vec<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        let mut paths = vec![PathBuf::from("/Applications/Claude.app")];
        if let Some(home) = dirs::home_dir() {
            paths.push(home.join("Applications/Claude.app"));
        }
        paths
    }
    #[cfg(windows)]
    {
        // The Squirrel install: a stub launcher at the top level and the real
        // binary in a versioned `app-<version>` directory beside it. The
        // versioned one is preferred — the stub has been reported to need
        // extra arguments of its own — and the newest is taken because
        // Squirrel leaves the previous version in place after an update.
        //
        // The Store build is deliberately absent; see `packaged_install`.
        let Some(root) = dirs::data_local_dir().map(|d| d.join("AnthropicClaude")) else {
            return Vec::new();
        };
        let mut paths = newest_squirrel_app(&root)
            .map(|dir| vec![dir.join("claude.exe")])
            .unwrap_or_default();
        paths.push(root.join("claude.exe"));
        paths
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        // The official Linux package installs `claude-desktop` on PATH. The
        // docs never give an absolute path, so PATH is the source of truth
        // and the fixed paths are only a fallback.
        let mut paths: Vec<PathBuf> = Vec::new();
        if let Some(found) = on_path("claude-desktop") {
            paths.push(found);
        }
        paths.push(PathBuf::from("/usr/bin/claude-desktop"));
        paths.push(PathBuf::from("/usr/local/bin/claude-desktop"));
        paths
    }
}

/// The command that opens `app` on `profile`.
pub fn launch_command(app: &Path, profile: &Path) -> Command {
    let (program, args) = launch_argv(app, profile, cfg!(target_os = "macos"));
    let mut cmd = Command::new(program);
    cmd.args(args);
    cmd
}

/// Program and arguments for a launch, split out so the argument order can be
/// tested for both platforms from either one.
///
/// macOS goes through `open -n`, which is what makes concurrent instances
/// possible: it starts a new process instead of activating the running one,
/// and hands everything after `--args` to the app. `.app` is a directory, so
/// there is nothing to execute directly there anyway.
///
/// Everywhere else `app` is a real executable and the switch goes straight to
/// it — a child process rather than a detached launch, which is why the
/// caller must not wait on it.
fn launch_argv(app: &Path, profile: &Path, via_open: bool) -> (PathBuf, Vec<String>) {
    let switch = format!("--user-data-dir={}", profile.display());
    if via_open {
        return (
            PathBuf::from("open"),
            vec![
                "-n".to_string(),
                "-a".to_string(),
                app.to_string_lossy().into_owned(),
                "--args".to_string(),
                switch,
            ],
        );
    }
    (app.to_path_buf(), vec![switch])
}

/// Whether the launch detaches on its own. `open` returns as soon as the app
/// is up, so waiting on it is cheap and reports real failures; running the
/// executable directly means waiting would block for as long as the app is
/// open.
pub fn launch_detaches() -> bool {
    cfg!(target_os = "macos")
}

/// The app's MCP servers and preferences, inside the profile directory. New
/// profiles start without one.
pub const CONFIG_FILE: &str = "claude_desktop_config.json";

#[derive(Debug, PartialEq)]
pub enum ClonePlan {
    /// Nothing to copy — the source has no config of its own.
    NoSource,
    /// The destination already has one; copying would discard it.
    Keep,
    Copy,
}

/// Whether to copy the config file. Split out from the copying so the
/// "don't silently discard what's already there" rule can be tested without
/// a filesystem.
pub fn plan_clone(source_exists: bool, dest_exists: bool, force: bool) -> ClonePlan {
    if !source_exists {
        return ClonePlan::NoSource;
    }
    if dest_exists && !force {
        return ClonePlan::Keep;
    }
    ClonePlan::Copy
}

/// Copy `CONFIG_FILE` from one profile into another, via a staging file so an
/// interrupted copy can't leave the destination holding half a config.
///
/// `fs::copy` carries the source's mode across, which matters here: the file
/// can hold MCP server credentials and is `0600` in the app's own profile.
pub fn clone_config(source: &Path, dest: &Path) -> io::Result<u64> {
    let src = source.join(CONFIG_FILE);
    let dst = dest.join(CONFIG_FILE);
    fs::create_dir_all(dest)?;
    let staged = dst.with_extension("json.part");
    let _ = fs::remove_file(&staged);
    let bytes = fs::copy(&src, &staged)?;
    fs::rename(&staged, &dst)?;
    Ok(bytes)
}

/// Whether the profile holds a signed-in session.
///
/// Only the presence of a credential is checked, not its validity — reading
/// the identity behind it needs the Keychain and is a separate step (#75).
/// `oauth:tokenCache` is the pre-V2 key and can be left behind as an empty
/// placeholder, hence the emptiness check on both.
pub fn is_signed_in(profile: &Path) -> bool {
    let Ok(raw) = fs::read_to_string(profile.join("config.json")) else {
        return false;
    };
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return false;
    };
    ["oauth:tokenCacheV2", "oauth:tokenCache"]
        .iter()
        .any(|key| {
            json.get(*key)
                .and_then(|v| v.as_str())
                .is_some_and(|v| !v.is_empty())
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_config(tag: &str) -> AppConfig {
        let base = std::env::temp_dir().join(format!("cc-desktop-{}-{}", tag, std::process::id()));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(&base).unwrap();
        AppConfig { base_dir: base }
    }

    #[test]
    fn profiles_live_next_to_accounts() {
        let c = temp_config("layout");
        assert_eq!(profiles_dir(&c), c.base_dir.join("desktop"));
        assert_eq!(profile_path(&c, "work"), c.base_dir.join("desktop/work"));
        let _ = fs::remove_dir_all(&c.base_dir);
    }

    #[test]
    fn listing_is_empty_when_nothing_was_ever_added() {
        // The desktop dir is created lazily, so a missing one is normal
        // rather than an error.
        let c = temp_config("empty");
        assert!(list_profiles(&c).unwrap().is_empty());
        let _ = fs::remove_dir_all(&c.base_dir);
    }

    #[test]
    fn listing_returns_directories_sorted_and_ignores_files() {
        let c = temp_config("list");
        fs::create_dir_all(profile_path(&c, "work")).unwrap();
        fs::create_dir_all(profile_path(&c, "personal")).unwrap();
        fs::write(profiles_dir(&c).join("stray.txt"), "").unwrap();
        assert_eq!(list_profiles(&c).unwrap(), vec!["personal", "work"]);
        let _ = fs::remove_dir_all(&c.base_dir);
    }

    fn write_config(profile: &Path, body: &str) {
        fs::create_dir_all(profile).unwrap();
        fs::write(profile.join("config.json"), body).unwrap();
    }

    #[test]
    fn a_fresh_profile_is_not_signed_in() {
        let c = temp_config("fresh");
        let p = profile_path(&c, "work");
        fs::create_dir_all(&p).unwrap();
        assert!(!is_signed_in(&p), "no config.json at all");
        write_config(&p, r#"{"locale":"en-US"}"#);
        assert!(!is_signed_in(&p), "config.json without a token");
        let _ = fs::remove_dir_all(&c.base_dir);
    }

    #[test]
    fn either_token_key_counts_as_signed_in() {
        let c = temp_config("token");
        let p = profile_path(&c, "work");
        write_config(&p, r#"{"oauth:tokenCacheV2":"djEw..."}"#);
        assert!(is_signed_in(&p), "V2 key");
        write_config(&p, r#"{"oauth:tokenCache":"djEw..."}"#);
        assert!(is_signed_in(&p), "pre-V2 key");
        let _ = fs::remove_dir_all(&c.base_dir);
    }

    #[test]
    fn an_empty_token_placeholder_is_not_signed_in() {
        // The V1 entry survives the V2 migration as an empty string.
        let c = temp_config("placeholder");
        let p = profile_path(&c, "work");
        write_config(&p, r#"{"oauth:tokenCache":"","locale":"en-US"}"#);
        assert!(!is_signed_in(&p));
        let _ = fs::remove_dir_all(&c.base_dir);
    }

    #[test]
    fn a_corrupt_config_is_not_signed_in() {
        let c = temp_config("corrupt");
        let p = profile_path(&c, "work");
        write_config(&p, "{not json");
        assert!(!is_signed_in(&p));
        let _ = fs::remove_dir_all(&c.base_dir);
    }

    // Real `ps -ax -o pid=,command=` output, trimmed: the app's own instance,
    // one launched on a profile, and two of the helpers that must not be
    // mistaken for either.
    const PS: &str = concat!(
        " 92746 /Applications/Claude.app/Contents/MacOS/Claude\n",
        " 93513 /Applications/Claude.app/Contents/MacOS/Claude --user-data-dir=/Users/me/.claude-switch/desktop/work\n",
        " 93000 /Applications/Claude.app/Contents/Frameworks/Claude Helper.app/Contents/MacOS/Claude Helper --type=gpu-process --user-data-dir=/Users/me/.claude-switch/desktop/work\n",
        " 93001 /Applications/Claude.app/Contents/Frameworks/Claude Helper.app/Contents/MacOS/Claude Helper --type=renderer\n",
        " 93002 /Applications/Claude.app/Contents/Frameworks/Claude Helper (Renderer).app/Contents/MacOS/Claude Helper (Renderer) --type=renderer\n",
        "  1234 /usr/bin/something-else\n",
        // A shell running a script that merely names the path, and this tool
        // being told where the app lives. Both mention it; neither is Claude.
        " 1235 /bin/zsh -c open -a /Applications/Claude.app/Contents/MacOS/Claude\n",
        " 1236 claude-acc desktop add work\n",
    );

    #[test]
    fn only_the_app_itself_counts_as_running() {
        // Helpers, and anything that merely mentions the path in its
        // arguments, are not the app — the executable is what decides.
        let found = parse_ps(PS);
        assert_eq!(found.len(), 2, "{:?}", found);
        assert_eq!(
            found[0],
            Instance {
                pid: 92746,
                profile: None
            }
        );
        assert_eq!(
            found[1],
            Instance {
                pid: 93513,
                profile: Some("work".to_string())
            }
        );
    }

    #[test]
    fn nothing_running_is_nothing_found() {
        assert!(parse_ps("").is_empty());
        assert!(parse_ps(" 1234 /usr/bin/something-else\n").is_empty());
    }

    #[test]
    fn a_profile_path_with_no_file_name_falls_back_to_the_path() {
        let line = " 42 /Applications/Claude.app/Contents/MacOS/Claude --user-data-dir=/\n";
        assert_eq!(parse_ps(line)[0].profile.as_deref(), Some("/"));
    }

    #[test]
    fn launch_passes_the_profile_after_args() {
        // Order matters: anything before `--args` is consumed by `open`
        // itself, so a misplaced switch would silently do nothing.
        //
        // `via_open` is passed explicitly rather than going through
        // `launch_command`, so the macOS form is asserted on every platform
        // — the point of splitting `launch_argv` out in the first place.
        let (program, args) = launch_argv(
            Path::new("/Applications/Claude.app"),
            Path::new("/tmp/p"),
            true,
        );
        assert_eq!(program, PathBuf::from("open"));
        assert_eq!(
            args,
            vec![
                "-n",
                "-a",
                "/Applications/Claude.app",
                "--args",
                "--user-data-dir=/tmp/p",
            ]
        );
    }

    #[test]
    fn launch_command_uses_the_form_this_platform_needs() {
        // The wiring between the two, asserted without hardcoding either
        // platform's answer into the expectation.
        let app = Path::new("/somewhere/Claude");
        let profile = Path::new("/tmp/p");
        let cmd = launch_command(app, profile);
        let (program, args) = launch_argv(app, profile, cfg!(target_os = "macos"));
        assert_eq!(cmd.get_program(), program.as_os_str());
        let actual: Vec<String> = cmd
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert_eq!(actual, args);
    }

    #[test]
    fn a_profile_path_with_a_space_stays_one_argument() {
        // No shell is involved, so the path needs no quoting — and must not
        // get any, or the app would look for a directory named `"..."`.
        // True of both launch forms, hence the loop.
        for via_open in [true, false] {
            let (_, args) = launch_argv(
                Path::new("/Applications/Claude.app"),
                Path::new("/tmp/a b"),
                via_open,
            );
            assert_eq!(args.last().unwrap(), "--user-data-dir=/tmp/a b");
        }
    }

    #[test]
    fn the_override_wins_over_the_usual_locations() {
        let c = temp_config("app");
        let real = c.base_dir.join("Claude.app");
        let other = c.base_dir.join("Other.app");
        fs::create_dir_all(&real).unwrap();
        fs::create_dir_all(&other).unwrap();

        let candidates = vec![other.clone()];
        assert_eq!(
            pick_app(Some(real.to_str().unwrap()), &candidates),
            Some(real)
        );
        assert_eq!(pick_app(None, &candidates), Some(other.clone()));
        assert_eq!(pick_app(Some(""), &candidates), Some(other));
        let _ = fs::remove_dir_all(&c.base_dir);
    }

    #[test]
    fn a_set_but_missing_override_finds_nothing() {
        // Falling back would launch the very app the override meant to
        // replace, and the typo would never surface.
        let c = temp_config("app-missing");
        let present = c.base_dir.join("Claude.app");
        fs::create_dir_all(&present).unwrap();
        assert_eq!(pick_app(Some("/no/such/app"), &[present]), None);
        let _ = fs::remove_dir_all(&c.base_dir);
    }

    #[test]
    fn cloning_needs_a_source_config() {
        assert_eq!(plan_clone(false, false, false), ClonePlan::NoSource);
        // Even --force can't copy a file that isn't there.
        assert_eq!(plan_clone(false, true, true), ClonePlan::NoSource);
    }

    #[test]
    fn an_existing_destination_config_is_kept_unless_forced() {
        // The file holds MCP servers someone configured by hand; silently
        // replacing it would be the worst possible default.
        assert_eq!(plan_clone(true, true, false), ClonePlan::Keep);
        assert_eq!(plan_clone(true, true, true), ClonePlan::Copy);
    }

    #[test]
    fn an_empty_destination_is_copied_into() {
        assert_eq!(plan_clone(true, false, false), ClonePlan::Copy);
    }

    #[test]
    fn clone_config_writes_the_file_and_leaves_no_staging_behind() {
        let c = temp_config("clone");
        let src = profile_path(&c, "source");
        let dst = profile_path(&c, "target");
        fs::create_dir_all(&src).unwrap();
        let body = r#"{"mcpServers":{"docker":{"command":"docker"}}}"#;
        fs::write(src.join(CONFIG_FILE), body).unwrap();

        let bytes = clone_config(&src, &dst).unwrap();
        assert_eq!(bytes as usize, body.len());
        assert_eq!(fs::read_to_string(dst.join(CONFIG_FILE)).unwrap(), body);
        assert!(
            !dst.join("claude_desktop_config.json.part").exists(),
            "staging file survived"
        );
        let _ = fs::remove_dir_all(&c.base_dir);
    }

    #[test]
    fn clone_config_replaces_rather_than_appends() {
        // Regression shape: a plain write over a longer existing file would
        // leave the old tail behind.
        let c = temp_config("clone-over");
        let src = profile_path(&c, "source");
        let dst = profile_path(&c, "target");
        fs::create_dir_all(&src).unwrap();
        fs::create_dir_all(&dst).unwrap();
        fs::write(src.join(CONFIG_FILE), "{}").unwrap();
        fs::write(dst.join(CONFIG_FILE), r#"{"mcpServers":{"old":{}}}"#).unwrap();

        clone_config(&src, &dst).unwrap();
        assert_eq!(fs::read_to_string(dst.join(CONFIG_FILE)).unwrap(), "{}");
        let _ = fs::remove_dir_all(&c.base_dir);
    }

    #[test]
    fn launch_runs_the_executable_directly_where_there_is_no_open() {
        // Windows and Linux have a real binary to run; `open` is macOS's.
        let (program, args) =
            launch_argv(Path::new("/usr/bin/claude-desktop"), Path::new("/p"), false);
        assert_eq!(program, PathBuf::from("/usr/bin/claude-desktop"));
        assert_eq!(args, vec!["--user-data-dir=/p"]);
    }

    #[test]
    fn the_newest_squirrel_app_directory_wins() {
        // Squirrel leaves the previous version in place after an update, so
        // "whatever read_dir happens to yield first" would be a coin flip
        // between the new binary and the old one.
        let c = temp_config("squirrel");
        let root = c.base_dir.join("AnthropicClaude");
        for name in ["app-1.7196.1", "app-1.24012.9", "packages", "Update.exe"] {
            fs::create_dir_all(root.join(name)).unwrap();
        }
        assert_eq!(
            newest_squirrel_app(&root),
            Some(root.join("app-1.7196.1")),
            "sorted by name, which is Squirrel's own ordering"
        );
        let _ = fs::remove_dir_all(&c.base_dir);
    }

    #[test]
    fn a_squirrel_root_without_app_directories_yields_nothing() {
        let c = temp_config("squirrel-empty");
        let root = c.base_dir.join("AnthropicClaude");
        fs::create_dir_all(root.join("packages")).unwrap();
        assert_eq!(newest_squirrel_app(&root), None);
        assert_eq!(newest_squirrel_app(&c.base_dir.join("nope")), None);
        let _ = fs::remove_dir_all(&c.base_dir);
    }

    #[test]
    fn a_store_package_directory_is_recognised() {
        // The version and hash of the package name vary, so only the prefix
        // can be matched — hardcoding the family name would go stale.
        let c = temp_config("packages");
        let packages = c.base_dir.join("Packages");
        fs::create_dir_all(packages.join("Microsoft.WindowsStore_8wekyb3d8bbwe")).unwrap();
        assert!(!has_claude_package(&packages), "unrelated packages only");
        fs::create_dir_all(packages.join("Claude_1.7196.1.0_x64__pzs8sxrjxfjjc")).unwrap();
        assert!(has_claude_package(&packages));
        let _ = fs::remove_dir_all(&c.base_dir);
    }

    #[test]
    fn a_missing_packages_root_is_not_a_store_install() {
        assert!(!has_claude_package(Path::new("/no/such/Packages")));
    }

    #[test]
    fn no_candidate_exists_means_no_app() {
        assert_eq!(pick_app(None, &[PathBuf::from("/no/such/app")]), None);
        assert_eq!(pick_app(None, &[]), None);
    }
}
