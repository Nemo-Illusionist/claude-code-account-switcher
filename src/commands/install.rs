use std::fs;
use std::path::PathBuf;
use crate::config::AppConfig;
use crate::i18n::{I18n, Msg};

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
                ensure_shell_integration(config, i18n);
                return;
            }
            i18n.print(Msg::InstallUpdating(installed_version.clone(), current_version.to_string()));
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

    i18n.print(Msg::InstallDone(
        target.to_str().unwrap_or("").to_string(),
    ));

    ensure_shell_integration(config, i18n);
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

        if content.contains("claude-acc init") {
            // Already has some form of claude-acc init
            if content.contains(bin_str) {
                i18n.print(Msg::InstallShellAlready(rc.to_string_lossy().to_string()));
            } else {
                // Update old eval line to point to new binary
                let updated = update_eval_line(&content, &eval_line);
                fs::write(&rc, updated).expect("Failed to update rc file");
                i18n.print(Msg::InstallShellUpdated(rc.to_string_lossy().to_string()));
            }
        } else {
            let mut content = content;
            if !content.ends_with('\n') && !content.is_empty() {
                content.push('\n');
            }
            content.push_str(&format!("\n# Claude Code Account Switcher\n{}\n", eval_line));
            fs::write(&rc, content).expect("Failed to update rc file");
            i18n.print(Msg::InstallShellAdded(rc.to_string_lossy().to_string()));
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
    if let Ok(profile) = std::env::var("PROFILE") {
        if !profile.is_empty() {
            return ("pwsh".to_string(), Some(PathBuf::from(profile)));
        }
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

fn update_eval_line(content: &str, new_eval: &str) -> String {
    let mut result = String::new();
    for line in content.lines() {
        if line.contains("claude-acc init") {
            result.push_str(new_eval);
        } else {
            result.push_str(line);
        }
        result.push('\n');
    }
    result
}
