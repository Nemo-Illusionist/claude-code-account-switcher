use crate::config::{AppConfig, validate_name};
use crate::environment::strip_claude_auth_env;
use crate::i18n::{I18n, Msg};
use crate::sessions::{self, SessionRef};
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
///
/// Also strips ANTHROPIC_API_KEY / ANTHROPIC_AUTH_TOKEN / CLAUDE_CODE_OAUTH_TOKEN /
/// AWS_BEARER_TOKEN_BEDROCK — any of these leaking in from the parent shell can
/// override which identity claude actually uses, regardless of CLAUDE_CONFIG_DIR.
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
    strip_claude_auth_env(&mut cmd);
    cmd
}

pub fn run(config: &AppConfig, i18n: &I18n, name: &str, args: &[String]) {
    if name == "default" {
        let dir = crate::identity::standard_token_dir();
        if let Some(dir) = dir.as_deref() {
            preflight_resume(config, i18n, args, sessions::DEFAULT_LABEL, dir);
        }
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
    preflight_resume(config, i18n, args, name, &acc_dir);
    let status = build_command(args, Some(&acc_dir))
        .status()
        .expect("Failed to run claude");
    std::process::exit(status.code().unwrap_or(1));
}

/// What to do about a `--resume <id>` that names a session the target account
/// may not have. Indices refer to the `copies` slice the plan was built from.
#[derive(Debug, PartialEq)]
enum ResumePlan {
    /// Either the target already holds the only relevant copy, or nothing
    /// anywhere matches — let claude handle it as it normally would.
    Proceed,
    /// Not in the target, and exactly one other account has it.
    Copy(usize),
    /// Not in the target, and several other accounts have it.
    Choose(Vec<usize>),
    /// The target has it, but another account's copy is newer.
    Newer(usize),
}

/// Before handing over to claude, check whether the requested `--resume`
/// session actually lives in the account we're about to run under. It usually
/// doesn't — sessions belong to the account that created them — and claude
/// would just report an unknown session with no hint that the transcript
/// exists one account over.
fn preflight_resume(
    config: &AppConfig,
    i18n: &I18n,
    args: &[String],
    target: &str,
    target_dir: &Path,
) {
    let Some(id) = resume_id(args) else {
        return;
    };
    let copies = sessions::find_by_id(config, id);
    let chosen = match plan_resume(&copies, target) {
        ResumePlan::Proceed => return,
        ResumePlan::Copy(i) => {
            println!();
            i18n.print(Msg::ResumeNotHere(id.to_string(), target.to_string()));
            println!("{}", super::session::describe(&copies[i], i18n));
            println!();
            i18n.print(Msg::SessionCostNote);
            let q = Msg::ResumeCopyConfirm(copies[i].account.clone(), target.to_string());
            if !super::session::confirm(i18n, q) {
                i18n.print(Msg::ResumeContinuingWithout);
                return;
            }
            i
        }
        ResumePlan::Choose(candidates) => {
            println!();
            i18n.print(Msg::ResumeNotHere(id.to_string(), target.to_string()));
            match super::session::ask_which(&copies, &candidates, i18n) {
                Some(i) => i,
                None => {
                    i18n.print(Msg::ResumeContinuingWithout);
                    return;
                }
            }
        }
        ResumePlan::Newer(i) => {
            println!();
            i18n.print(Msg::ResumeNewerElsewhere(copies[i].account.clone()));
            for c in &copies {
                println!("{}", super::session::describe(c, i18n));
            }
            println!();
            let q = Msg::ResumeUseNewerConfirm(copies[i].account.clone());
            if !super::session::confirm(i18n, q) {
                i18n.print(Msg::ResumeContinuingLocal(target.to_string()));
                return;
            }
            i
        }
    };

    match sessions::copy_into(&copies[chosen], target_dir) {
        Ok(_) => i18n.print(Msg::ResumeCopied(
            copies[chosen].account.clone(),
            target.to_string(),
        )),
        Err(e) => {
            i18n.print(Msg::SessionCopyFailed(e.to_string()));
            i18n.print(Msg::ResumeContinuingWithout);
        }
    }
}

/// The session id in a forwarded `--resume <id>`, `--resume=<id>` or
/// `-r <id>`. A bare `--resume` (claude's interactive session picker) yields
/// `None`: there is no id to look up, and stepping in front of the picker
/// would only get in the way.
fn resume_id(args: &[String]) -> Option<&str> {
    for (i, arg) in args.iter().enumerate() {
        if let Some(v) = arg.strip_prefix("--resume=") {
            return (!v.is_empty()).then_some(v);
        }
        if arg == "--resume" || arg == "-r" {
            let next = args.get(i + 1)?;
            // A flag after `--resume` means the picker form, not an id.
            return (!next.starts_with('-')).then_some(next.as_str());
        }
    }
    None
}

/// Decide whether a cross-account copy is worth offering. Pure, so every
/// branch is testable without touching a filesystem or a prompt.
fn plan_resume(copies: &[SessionRef], target: &str) -> ResumePlan {
    let elsewhere: Vec<usize> = copies
        .iter()
        .enumerate()
        .filter(|(_, s)| s.account != target)
        .map(|(i, _)| i)
        .collect();
    if elsewhere.is_empty() {
        return ResumePlan::Proceed;
    }

    let newest_elsewhere = elsewhere
        .iter()
        .copied()
        .max_by_key(|&i| copies[i].modified)
        .expect("non-empty");

    match copies.iter().find(|s| s.account == target) {
        // Only offer to pull in another account's copy when it is genuinely
        // ahead of the local one — an older copy elsewhere is just history.
        Some(local) if copies[newest_elsewhere].modified > local.modified => {
            ResumePlan::Newer(newest_elsewhere)
        }
        Some(_) => ResumePlan::Proceed,
        None if elsewhere.len() == 1 => ResumePlan::Copy(elsewhere[0]),
        None => ResumePlan::Choose(elsewhere),
    }
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

    fn args(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn resume_id_reads_the_value_after_the_flag() {
        assert_eq!(resume_id(&args(&["--resume", "abc"])), Some("abc"));
        assert_eq!(resume_id(&args(&["-r", "abc"])), Some("abc"));
        assert_eq!(resume_id(&args(&["--resume=abc"])), Some("abc"));
    }

    #[test]
    fn resume_id_ignores_the_interactive_picker_form() {
        // A bare `--resume` opens claude's own session picker; there is no id
        // to look up and we must not step in front of it.
        assert_eq!(resume_id(&args(&["--resume"])), None);
        assert_eq!(resume_id(&args(&["--resume", "--verbose"])), None);
        assert_eq!(resume_id(&args(&["--resume="])), None);
    }

    #[test]
    fn resume_id_none_without_the_flag() {
        assert_eq!(resume_id(&args(&[])), None);
        assert_eq!(resume_id(&args(&["--continue"])), None);
        assert_eq!(resume_id(&args(&["-p", "hello"])), None);
    }

    #[test]
    fn resume_id_finds_the_flag_after_other_arguments() {
        assert_eq!(
            resume_id(&args(&["--verbose", "--resume", "abc"])),
            Some("abc")
        );
    }

    fn session(account: &str, modified: u64) -> SessionRef {
        SessionRef {
            id: "s".to_string(),
            account: account.to_string(),
            project: "proj".to_string(),
            path: std::path::PathBuf::from("/tmp/s.jsonl"),
            modified: Some(modified),
            size: 10,
        }
    }

    #[test]
    fn nothing_to_offer_when_no_other_account_has_it() {
        assert_eq!(plan_resume(&[], "work"), ResumePlan::Proceed);
        assert_eq!(
            plan_resume(&[session("work", 100)], "work"),
            ResumePlan::Proceed
        );
    }

    #[test]
    fn offers_the_single_copy_held_elsewhere() {
        let copies = vec![session("personal", 100)];
        assert_eq!(plan_resume(&copies, "work"), ResumePlan::Copy(0));
    }

    #[test]
    fn asks_which_when_several_accounts_hold_it() {
        let copies = vec![session("personal", 100), session("other", 200)];
        assert_eq!(plan_resume(&copies, "work"), ResumePlan::Choose(vec![0, 1]));
    }

    #[test]
    fn offers_a_newer_copy_from_another_account() {
        let copies = vec![session("work", 100), session("personal", 500)];
        assert_eq!(plan_resume(&copies, "work"), ResumePlan::Newer(1));
    }

    #[test]
    fn stays_quiet_when_the_local_copy_is_the_newest() {
        // An older copy elsewhere is just history — nothing to decide.
        let copies = vec![session("work", 500), session("personal", 100)];
        assert_eq!(plan_resume(&copies, "work"), ResumePlan::Proceed);
    }

    #[test]
    fn equal_timestamps_are_not_treated_as_newer() {
        let copies = vec![session("work", 100), session("personal", 100)];
        assert_eq!(plan_resume(&copies, "work"), ResumePlan::Proceed);
    }

    #[test]
    fn newer_offer_picks_the_freshest_of_several_elsewhere() {
        let copies = vec![
            session("work", 100),
            session("personal", 200),
            session("other", 900),
        ];
        assert_eq!(plan_resume(&copies, "work"), ResumePlan::Newer(2));
    }

    #[test]
    fn strips_auth_env_vars_that_could_override_the_selected_account() {
        for acc_dir in [None, Some(Path::new("/tmp/some-account"))] {
            let cmd = build_command(&[], acc_dir);
            for var in crate::environment::CLAUDE_AUTH_ENV_VARS {
                let removed = cmd
                    .get_envs()
                    .find(|(k, _)| *k == std::ffi::OsStr::new(*var));
                assert_eq!(
                    removed,
                    Some((std::ffi::OsStr::new(*var), None)),
                    "{var} not stripped for acc_dir={acc_dir:?}"
                );
            }
        }
    }
}
