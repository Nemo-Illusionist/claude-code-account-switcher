use crate::config::AppConfig;
use crate::i18n::{I18n, Msg};
use crate::ide;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Filename the installed binary lives under inside `~/.claude-switch/bin/`.
/// Windows requires the `.exe` extension or the OS won't execute the file
/// even when the path is given explicitly.
fn binary_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "claude-acc.exe"
    } else {
        "claude-acc"
    }
}

pub fn run(config: &AppConfig, i18n: &I18n) {
    let bin_dir = config.base_dir.join("bin");
    fs::create_dir_all(&bin_dir).expect("Failed to create bin directory");

    let target = bin_dir.join(binary_name());
    let source = std::env::current_exe().expect("Cannot determine binary path");

    // On Windows, prior versions copied the binary as `claude-acc` (no
    // extension) which Windows can't execute. Clean that up so PATH-based
    // lookups stop hitting the broken file.
    if cfg!(target_os = "windows") {
        let stale = bin_dir.join("claude-acc");
        if stale.is_file() && stale != target {
            let _ = fs::remove_file(&stale);
        }
    }

    // Check version
    let current_version = env!("CARGO_PKG_VERSION");

    if target.exists() {
        // Run the installed binary with --version to get its version
        let output = std::process::Command::new(&target)
            .arg("--version")
            .output();

        if let Ok(output) = output {
            let installed_version = String::from_utf8_lossy(&output.stdout);
            // Format: "claude-acc X.Y.Z\n"
            let installed_version = installed_version.trim().replace("claude-acc ", "");
            if installed_version == current_version {
                i18n.print(Msg::InstallUpToDate(current_version.to_string()));
                ensure_ide_integration(config, &target);
                ensure_shell_integration(config, i18n);
                return;
            }
            i18n.print(Msg::InstallUpdating(
                installed_version.clone(),
                current_version.to_string(),
            ));
        }
    } else {
        i18n.print(Msg::InstallCopying(current_version.to_string()));
    }

    fs::copy(&source, &target).expect("Failed to copy binary");

    // Make executable on unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&target, fs::Permissions::from_mode(0o755)).ok();
    }

    i18n.print(Msg::InstallDone(target.to_str().unwrap_or("").to_string()));

    ensure_ide_integration(config, &target);
    ensure_shell_integration(config, i18n);
}

fn ensure_ide_integration(config: &AppConfig, claude_acc_bin: &Path) {
    // Best-effort: errors here shouldn't block install. The shell integration
    // and binary copy already happened.
    let _ = ide::install_wrapper(config, claude_acc_bin);
    let _ = ide::refresh_all_account_symlinks(config);
}

fn ensure_shell_integration(config: &AppConfig, i18n: &I18n) {
    let bin_path = config.base_dir.join("bin").join(binary_name());
    let bin_str = bin_path.to_str().expect("Invalid bin path");

    let (shell, rc_path) = detect_shell_and_rc();

    let eval_line = match shell.as_str() {
        "pwsh" | "powershell" => format!("Invoke-Expression (& '{}' init pwsh)", bin_str),
        _ => format!("eval \"$('{0}' init {1})\"", bin_str, shell),
    };

    if let Some(rc) = rc_path {
        // PowerShell profile paths can point at $HOME/Documents/PowerShell/...
        // which may not exist on a fresh install. Create parent dirs lazily.
        if let Some(parent) = rc.parent()
            && !parent.as_os_str().is_empty()
        {
            let _ = fs::create_dir_all(parent);
        }
        let content = fs::read_to_string(&rc).unwrap_or_default();

        // Match any line that mentions claude-acc + init (handles quoting
        // around the binary path, like 'claude-acc' init zsh).
        let init_line_count = content
            .lines()
            .filter(|l| is_claude_acc_init_line(l))
            .count();
        let has_exact_match = content.lines().any(|l| l.trim() == eval_line.trim());

        if init_line_count == 0 {
            let mut content = content;
            if !content.ends_with('\n') && !content.is_empty() {
                content.push('\n');
            }
            content.push_str(&format!(
                "\n# Claude Code Account Switcher\n{}\n",
                eval_line
            ));
            fs::write(&rc, content).expect("Failed to update rc file");
            i18n.print(Msg::InstallShellAdded(rc.to_string_lossy().to_string()));
        } else if init_line_count == 1 && has_exact_match {
            i18n.print(Msg::InstallShellAlready(rc.to_string_lossy().to_string()));
        } else {
            // Either: stale path (mismatch) or duplicates from a previous
            // buggy install. Either way, dedupe and refresh.
            let updated = update_eval_line(&content, &eval_line);
            fs::write(&rc, updated).expect("Failed to update rc file");
            i18n.print(Msg::InstallShellUpdated(rc.to_string_lossy().to_string()));
        }
    } else {
        i18n.print(Msg::InstallShellManual(eval_line));
    }
}

fn detect_shell_and_rc() -> (String, Option<PathBuf>) {
    if cfg!(target_os = "windows") {
        return detect_shell_windows();
    }
    detect_shell_unix()
}

fn detect_shell_unix() -> (String, Option<PathBuf>) {
    let home = dirs::home_dir().expect("Cannot determine home directory");

    let shell_env = std::env::var("SHELL").unwrap_or_default();
    if shell_env.contains("zsh") {
        return ("zsh".to_string(), Some(home.join(".zshrc")));
    }
    if shell_env.contains("bash") {
        let bashrc = home.join(".bashrc");
        let profile = home.join(".bash_profile");
        let rc = if bashrc.exists() { bashrc } else { profile };
        return ("bash".to_string(), Some(rc));
    }

    // Fallback: check common rc files
    if home.join(".zshrc").exists() {
        return ("zsh".to_string(), Some(home.join(".zshrc")));
    }
    if home.join(".bashrc").exists() {
        return ("bash".to_string(), Some(home.join(".bashrc")));
    }

    ("bash".to_string(), None)
}

/// Windows: PowerShell is the only target shell we support. Git Bash etc.
/// would appear with a unix-style $SHELL but on Windows we prefer pwsh,
/// because that's where IDEs and the standard terminal land.
///
/// Resolution order for the profile path:
///   1. `pwsh -NoProfile -Command "$PROFILE"` (PowerShell 7+)
///   2. `powershell -NoProfile -Command "$PROFILE"` (Windows PowerShell 5.x)
///   3. Hardcoded `~/Documents/PowerShell/Microsoft.PowerShell_profile.ps1`
///
/// The `$PROFILE` automatic variable in PowerShell is *not* exported to
/// child processes by default, so we can't read it via `std::env::var`.
fn detect_shell_windows() -> (String, Option<PathBuf>) {
    for shell_bin in ["pwsh", "powershell"] {
        if let Some(p) = pwsh_profile_path(shell_bin) {
            return ("pwsh".to_string(), Some(p));
        }
    }
    // Last-resort fallback: standard PSCore profile location, even if pwsh
    // isn't on PATH. The user can always source it manually.
    let home = dirs::home_dir().expect("Cannot determine home directory");
    let fallback = home
        .join("Documents")
        .join("PowerShell")
        .join("Microsoft.PowerShell_profile.ps1");
    ("pwsh".to_string(), Some(fallback))
}

fn pwsh_profile_path(shell_bin: &str) -> Option<PathBuf> {
    let out = Command::new(shell_bin)
        .args(["-NoProfile", "-NonInteractive", "-Command", "$PROFILE"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if path.is_empty() {
        return None;
    }
    Some(PathBuf::from(path))
}

fn is_claude_acc_init_line(line: &str) -> bool {
    line.contains("claude-acc")
        && line.contains("init")
        && (line.contains("eval") || line.contains("Invoke-Expression"))
}

const HEADER_COMMENT: &str = "# Claude Code Account Switcher";

fn update_eval_line(content: &str, new_eval: &str) -> String {
    // Replace the first matching init line; drop any subsequent duplicates
    // and their accompanying header comment (older versions of `install`
    // would append rather than dedupe).
    let mut out: Vec<String> = Vec::new();
    let mut replaced = false;
    for line in content.lines() {
        if is_claude_acc_init_line(line) {
            if !replaced {
                out.push(new_eval.to_string());
                replaced = true;
            } else if out.last().map(|s| s.trim()) == Some(HEADER_COMMENT) {
                // Duplicate: drop both this eval line and its preceding
                // header comment so we don't leave an orphan.
                out.pop();
            }
        } else {
            out.push(line.to_string());
        }
    }
    let mut result = out.join("\n");
    result.push('\n');
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(target_os = "windows")]
    fn binary_name_has_exe_on_windows() {
        assert_eq!(binary_name(), "claude-acc.exe");
    }

    #[test]
    #[cfg(not(target_os = "windows"))]
    fn binary_name_has_no_extension_elsewhere() {
        assert_eq!(binary_name(), "claude-acc");
    }

    #[test]
    fn detects_quoted_path_eval_line() {
        assert!(is_claude_acc_init_line(
            r#"eval "$('/Users/me/.claude-switch/bin/claude-acc' init zsh)""#
        ));
    }

    #[test]
    fn detects_powershell_invocation() {
        assert!(is_claude_acc_init_line(
            "Invoke-Expression (& 'C:\\Users\\me\\.claude-switch\\bin\\claude-acc' init pwsh)"
        ));
    }

    #[test]
    fn ignores_unrelated_lines() {
        assert!(!is_claude_acc_init_line("# Claude Code Account Switcher"));
        assert!(!is_claude_acc_init_line("alias claudey='claude --foo'"));
        assert!(!is_claude_acc_init_line("export PATH=/some/bin:$PATH"));
    }

    #[test]
    fn ignores_partial_match_without_eval() {
        // mentions claude-acc and "init" but not as an eval line
        assert!(!is_claude_acc_init_line(
            "# claude-acc init zsh runs on shell startup"
        ));
    }

    #[test]
    fn update_eval_line_replaces_single_match() {
        let input = "\
# unrelated
eval \"$('/old/path/claude-acc' init zsh)\"
# trailing
";
        let new_eval = "eval \"$('/new/path/claude-acc' init zsh)\"";
        let out = update_eval_line(input, new_eval);
        assert!(out.contains("/new/path/claude-acc"));
        assert!(!out.contains("/old/path/claude-acc"));
        assert!(out.contains("# unrelated"));
        assert!(out.contains("# trailing"));
    }

    #[test]
    fn update_eval_line_dedupes_multiple_matches_and_drops_orphan_headers() {
        let input = "\
# preamble

# Claude Code Account Switcher
eval \"$('/p1/claude-acc' init zsh)\"

# Claude Code Account Switcher
eval \"$('/p2/claude-acc' init zsh)\"

# Claude Code Account Switcher
eval \"$('/p3/claude-acc' init zsh)\"
";
        let new_eval = "eval \"$('/new/claude-acc' init zsh)\"";
        let out = update_eval_line(input, new_eval);

        // Exactly one eval line, exactly one header.
        assert_eq!(out.matches("eval \"$(").count(), 1, "got: {out}");
        assert_eq!(
            out.matches("# Claude Code Account Switcher").count(),
            1,
            "got: {out}"
        );
        assert!(out.contains("/new/claude-acc"));
    }
}
