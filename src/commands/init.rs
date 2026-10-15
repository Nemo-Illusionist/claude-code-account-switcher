use crate::quoting::{is_single_line, pwsh_single_quoted, sh_single_quoted};

pub fn run(shell: &str) {
    let bin = std::env::current_exe()
        .expect("Cannot determine binary path")
        .to_str()
        .expect("Invalid binary path")
        .to_string();

    match render(shell, &bin) {
        Ok(out) => print!("{}", out),
        Err(msg) => {
            eprintln!("{}", msg);
            std::process::exit(1);
        }
    }
}

/// The shell integration for `shell`, with `bin` substituted in.
///
/// Separated from `run` so the generated text can be handed to the shell's
/// own parser in tests instead of asserting on strings — reading
/// `current_exe()` and printing are the only things left outside.
fn render(shell: &str, bin: &str) -> Result<String, String> {
    // Each template wraps the placeholder in that shell's single quotes, and
    // the two shells escape a quote inside them differently — so the escaper
    // is picked alongside the template rather than applied once afterwards.
    // Using the POSIX rule on `init.ps1` would break it in a new way rather
    // than fix it (#101).
    let (template, quote): (&str, fn(&str) -> String) = match shell {
        "zsh" => (include_str!("../../shell/init.zsh"), sh_single_quoted),
        "bash" => (include_str!("../../shell/init.bash"), sh_single_quoted),
        "pwsh" | "powershell" => (include_str!("../../shell/init.ps1"), pwsh_single_quoted),
        other => {
            return Err(format!(
                "Unsupported shell: {}. Use: zsh, bash, pwsh",
                other
            ));
        }
    };

    // Quoting cannot rescue a newline: these templates are line-oriented, so
    // a path containing one would be split across lines and mis-parsed while
    // `init` still exited 0.
    if !is_single_line(bin) {
        return Err(format!(
            "Refusing to emit shell integration: the path to this binary contains a newline ({:?})",
            bin
        ));
    }

    Ok(template.replace("__CLAUDE_ACC_BIN__", &quote(bin)))
}

#[cfg(test)]
mod tests {
    use super::*;

    const APOSTROPHE: &str = "/Users/O'Brien/.claude-switch/bin/claude-acc";

    #[test]
    fn an_unknown_shell_is_refused_by_name() {
        let err = render("fish", "/bin/claude-acc").unwrap_err();
        assert!(err.contains("fish"), "{err}");
    }

    #[test]
    fn a_newline_in_the_path_is_refused_rather_than_emitted() {
        for shell in ["zsh", "bash", "pwsh"] {
            assert!(
                render(shell, "/Users/two\nlines/claude-acc").is_err(),
                "{shell} emitted a path containing a newline"
            );
        }
    }

    // Regression (#101): `/Users/O'Brien` is an ordinary home directory, and
    // every template pasted the path between single quotes unescaped. The
    // placeholder check is the cheap half; the parser checks below are what
    // actually prove it.
    #[test]
    fn the_placeholder_is_always_replaced() {
        for shell in ["zsh", "bash", "pwsh", "powershell"] {
            let out = render(shell, APOSTROPHE).unwrap();
            assert!(
                !out.contains("__CLAUDE_ACC_BIN__"),
                "{shell} left the placeholder behind"
            );
            assert!(out.contains("O'"), "{shell} lost the path: {out:.120}");
        }
    }

    // Each shell escapes a quote inside '...' its own way, so the outputs
    // must differ. Sharing one escaper would silently break one of them.
    #[test]
    fn posix_and_powershell_escape_differently() {
        let posix = render("zsh", APOSTROPHE).unwrap();
        let pwsh = render("pwsh", APOSTROPHE).unwrap();
        assert!(posix.contains(r"O'\''Brien"), "posix: {posix:.200}");
        assert!(pwsh.contains("O''Brien"), "pwsh: {pwsh:.200}");
        assert!(!pwsh.contains(r"O'\''Brien"), "POSIX rule leaked into ps1");
    }
}

// The real proof for #101 is not a string assertion but each shell's own
// parser refusing or accepting the file. Gated as a whole module rather than
// per-test: `zsh`/`bash` are not on the Windows runner, and leaving the
// helper behind there would be dead code that `-D warnings` fails on
// (see `.claude/rules/platform-gating.md`).
#[cfg(all(test, unix))]
mod parser_tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::process::Command;

    const APOSTROPHE: &str = "/Users/O'Brien/.claude-switch/bin/claude-acc";

    fn write_rendered(shell: &str, name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("cc-init-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        fs::write(&path, render(shell, APOSTROPHE).unwrap()).unwrap();
        path
    }

    /// `true` if the interpreter parsed the file, `None` if it isn't installed.
    fn parses(program: &str, args: &[&str]) -> Option<bool> {
        let out = Command::new(program).args(args).output().ok()?;
        Some(out.status.success())
    }

    #[test]
    fn zsh_parses_an_init_script_for_a_path_with_an_apostrophe() {
        let f = write_rendered("zsh", "init.zsh");
        let Some(ok) = parses("zsh", &["-n", f.to_str().unwrap()]) else {
            return; // no zsh here; the bash case below still covers the rule
        };
        assert!(ok, "zsh rejected {}", f.display());
    }

    #[test]
    fn bash_parses_an_init_script_for_a_path_with_an_apostrophe() {
        let f = write_rendered("bash", "init.bash");
        let Some(ok) = parses("bash", &["-n", f.to_str().unwrap()]) else {
            return;
        };
        assert!(ok, "bash rejected {}", f.display());
    }

    // PowerShell is the case a shared POSIX escaper would have broken, so it
    // gets the same treatment where pwsh is available.
    #[test]
    fn pwsh_parses_an_init_profile_for_a_path_with_an_apostrophe() {
        let f = write_rendered("pwsh", "init.ps1");
        let script = format!(
            "$e = $null; \
             [System.Management.Automation.Language.Parser]::ParseFile('{}', [ref]$null, [ref]$e) \
             | Out-Null; if ($e) {{ exit 1 }} else {{ exit 0 }}",
            f.to_str().unwrap()
        );
        let Some(ok) = parses("pwsh", &["-NoProfile", "-Command", &script]) else {
            return;
        };
        assert!(ok, "pwsh reported parse errors for {}", f.display());
    }
}
