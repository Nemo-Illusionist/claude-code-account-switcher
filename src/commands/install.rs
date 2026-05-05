use crate::config::AppConfig;
use crate::i18n::{I18n, Msg};
use crate::ide;
use std::fs;
use std::path::{Path, PathBuf};

pub fn run(config: &AppConfig, i18n: &I18n) {
    let bin_dir = config.base_dir.join("bin");
    fs::create_dir_all(&bin_dir).expect("Failed to create bin directory");

    let target = bin_dir.join("claude-acc");
    let source = std::env::current_exe().expect("Cannot determine binary path");

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
    let bin_path = config.base_dir.join("bin").join("claude-acc");
    let bin_str = bin_path.to_str().expect("Invalid bin path");

    let (shell, rc_path) = detect_shell_and_rc();

    let eval_line = match shell.as_str() {
        "pwsh" | "powershell" => format!("Invoke-Expression (& '{}' init pwsh)", bin_str),
        _ => format!("eval \"$('{0}' init {1})\"", bin_str, shell),
    };

    if let Some(rc) = rc_path {
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
    let home = dirs::home_dir().expect("Cannot determine home directory");

    // Check SHELL env var
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

    // Check for PowerShell profile
    if let Ok(profile) = std::env::var("PROFILE")
        && !profile.is_empty()
    {
        return ("pwsh".to_string(), Some(PathBuf::from(profile)));
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
