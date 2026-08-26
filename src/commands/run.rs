use crate::config::{AppConfig, validate_name};
use crate::i18n::{I18n, Msg};
use std::path::Path;
use std::process::Command;

/// Builds the `claude` invocation for `run`. For the "default" account this
/// must strip any inherited `CLAUDE_CONFIG_DIR` (e.g. exported by the shell's
/// directory-link hook) so `claude-acc run default` really runs the standard
/// ~/.claude/ account rather than whatever the current directory is linked to.
///
/// `claude` on PATH usually resolves to claude-acc's own IDE wrapper
/// (~/.claude-switch/bin/claude, see src/ide.rs), which re-derives
/// CLAUDE_CONFIG_DIR from $PWD whenever it finds the var unset — that would
/// silently undo the "default" request in a linked directory. CLAUDE_ACC_RUN_DEFAULT
/// tells the wrapper this is an explicit default run so it skips that step.
fn build_command(args: &[String], acc_dir: Option<&Path>) -> Command {
    let mut cmd = Command::new("claude");
    cmd.args(args);
    match acc_dir {
        Some(dir) => {
            cmd.env("CLAUDE_CONFIG_DIR", dir);
            cmd.env_remove("CLAUDE_ACC_RUN_DEFAULT");
        }
        None => {
            cmd.env_remove("CLAUDE_CONFIG_DIR");
            cmd.env("CLAUDE_ACC_RUN_DEFAULT", "1");
        }
    }
    cmd
}

pub fn run(config: &AppConfig, i18n: &I18n, name: &str, args: &[String]) {
    if name == "default" {
        let status = build_command(args, None)
            .status()
            .expect("Failed to run claude");
        std::process::exit(status.code().unwrap_or(1));
    }

    if !validate_name(name) {
        i18n.print(Msg::NameInvalid);
        std::process::exit(1);
    }

    if !config.account_exists(name) {
        i18n.print(Msg::LoginNotFound(name.to_string()));
        std::process::exit(1);
    }

    let acc_dir = config.account_path(name);
    let status = build_command(args, Some(&acc_dir))
        .status()
        .expect("Failed to run claude");
    std::process::exit(status.code().unwrap_or(1));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_account_strips_inherited_config_dir() {
        // Simulate the shell hook having exported CLAUDE_CONFIG_DIR for a
        // linked directory before `claude-acc run default` is invoked.
        unsafe {
            std::env::set_var("CLAUDE_CONFIG_DIR", "/tmp/some-linked-account");
        }

        let cmd = build_command(&[], None);
        let removed = cmd
            .get_envs()
            .find(|(k, _)| *k == std::ffi::OsStr::new("CLAUDE_CONFIG_DIR"));

        unsafe {
            std::env::remove_var("CLAUDE_CONFIG_DIR");
        }

        // env_remove() records the key with a `None` value so the child
        // process never sees it, regardless of what the parent inherited.
        assert_eq!(
            removed,
            Some((std::ffi::OsStr::new("CLAUDE_CONFIG_DIR"), None))
        );
    }

    #[test]
    fn default_account_marks_run_default_for_the_ide_wrapper() {
        // The `claude` on PATH is usually claude-acc's own IDE wrapper,
        // which re-derives CLAUDE_CONFIG_DIR from $PWD when it sees the var
        // unset. This marker tells it to skip that for an explicit default run.
        let cmd = build_command(&[], None);
        let marker = cmd
            .get_envs()
            .find(|(k, _)| *k == std::ffi::OsStr::new("CLAUDE_ACC_RUN_DEFAULT"));

        assert_eq!(
            marker,
            Some((
                std::ffi::OsStr::new("CLAUDE_ACC_RUN_DEFAULT"),
                Some(std::ffi::OsStr::new("1"))
            ))
        );
    }

    #[test]
    fn named_account_sets_config_dir() {
        let dir = Path::new("/tmp/some-account");
        let cmd = build_command(&[], Some(dir));
        let set = cmd
            .get_envs()
            .find(|(k, _)| *k == std::ffi::OsStr::new("CLAUDE_CONFIG_DIR"));

        assert_eq!(
            set,
            Some((
                std::ffi::OsStr::new("CLAUDE_CONFIG_DIR"),
                Some(std::ffi::OsStr::new("/tmp/some-account"))
            ))
        );
    }

    #[test]
    fn named_account_clears_run_default_marker() {
        let dir = Path::new("/tmp/some-account");
        let cmd = build_command(&[], Some(dir));
        let marker = cmd
            .get_envs()
            .find(|(k, _)| *k == std::ffi::OsStr::new("CLAUDE_ACC_RUN_DEFAULT"));

        assert_eq!(
            marker,
            Some((std::ffi::OsStr::new("CLAUDE_ACC_RUN_DEFAULT"), None))
        );
    }
}
