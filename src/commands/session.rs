use crate::config::{AppConfig, validate_name};
use crate::i18n::{I18n, Msg};
use crate::identity;
use crate::sessions::{self, SessionRef};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// Which copy of a session `copy` should read from.
#[derive(Debug, PartialEq)]
enum Source {
    /// Exactly one candidate — no question to ask.
    One(usize),
    /// Several accounts hold this id; the user has to choose.
    Ambiguous(Vec<usize>),
    /// `--from` named an account that holds no copy of this id.
    UnknownFrom,
    /// The only copy is already in the destination account.
    AlreadyThere,
}

pub fn copy(
    config: &AppConfig,
    i18n: &I18n,
    id: &str,
    to: &str,
    from: Option<&str>,
    force: bool,
) -> i32 {
    let Some(dest_dir) = account_config_dir(config, to) else {
        i18n.print(Msg::LoginNotFound(to.to_string()));
        return 1;
    };

    let copies = sessions::find_by_id(config, id);
    if copies.is_empty() {
        i18n.print(Msg::SessionNotFound(id.to_string()));
        return 1;
    }

    let src = match pick_source(&copies, to, from) {
        Source::One(i) => &copies[i],
        Source::UnknownFrom => {
            i18n.print(Msg::SessionSourceUnknown(
                from.unwrap_or("").to_string(),
                id.to_string(),
            ));
            return 1;
        }
        Source::AlreadyThere => {
            i18n.print(Msg::SessionAlreadyThere(id.to_string(), to.to_string()));
            return 1;
        }
        Source::Ambiguous(candidates) => {
            if force {
                // Nothing to disambiguate with, and picking silently would
                // risk copying the wrong side of a drifted pair.
                i18n.print(Msg::SessionAmbiguousNeedsFrom(id.to_string()));
                return 1;
            }
            println!();
            i18n.print(Msg::SessionPickSource);
            match ask_which(&copies, &candidates, i18n, None) {
                Some(i) => &copies[i],
                None => {
                    i18n.print(Msg::SessionCancelled);
                    return 1;
                }
            }
        }
    };

    let existing = copies.iter().find(|s| s.account == to);
    if let Some(old) = existing
        && !force
    {
        println!();
        i18n.print(Msg::SessionOverwriteWarn(to.to_string()));
        println!(
            "{}  {}",
            describe(src, i18n),
            i18n.msg(Msg::SessionLabelSource)
        );
        println!(
            "{}  {}",
            describe(old, i18n),
            i18n.msg(Msg::SessionLabelReplaced)
        );
    }

    if !force {
        println!();
        i18n.print(Msg::SessionCostNote);
        let question = if existing.is_some() {
            Msg::SessionConfirmOverwrite(src.account.clone(), to.to_string())
        } else {
            Msg::SessionConfirmCopy(src.account.clone(), to.to_string())
        };
        if !confirm(i18n, question) {
            i18n.print(Msg::SessionCancelled);
            return 1;
        }
    }

    match sessions::copy_into(src, &dest_dir) {
        Ok(report) => {
            i18n.print(Msg::SessionCopied(
                id.to_string(),
                src.account.clone(),
                to.to_string(),
                sessions::human_size(report.bytes),
            ));
            if report.subagents > 0 {
                i18n.print(Msg::SessionCopiedSubagents(report.subagents));
            }
            i18n.print(Msg::SessionResumeHint(to.to_string(), id.to_string()));
            0
        }
        Err(e) => {
            i18n.print(Msg::SessionCopyFailed(e.to_string()));
            1
        }
    }
}

/// What to do about a `--resume <id>` that names a session the target account
/// may not have. Indices refer to the `copies` slice the plan was built from.
#[derive(Debug, PartialEq)]
enum ResumePlan {
    /// No other account has this session — let claude handle it as it
    /// normally would, including reporting an id that exists nowhere.
    Proceed,
    /// Not in the target account, and exactly one other account has it.
    Copy(usize),
    /// More than one copy is in play: either several other accounts have it,
    /// or the target has one *and* another account does. Both are a question
    /// for the user, so when the target has a copy the candidate list carries
    /// it too — picking it means "keep what's already here".
    Choose(Vec<usize>),
}

/// Before handing over to claude, check whether the requested `--resume`
/// session actually lives in the account we're about to run under. It usually
/// doesn't — sessions belong to the account that created them — and claude
/// would just report an unknown session with no hint that the transcript
/// exists one account over.
pub fn preflight_resume(
    config: &AppConfig,
    i18n: &I18n,
    args: &[String],
    target: &str,
    target_dir: &Path,
) {
    let Some(id) = resume_id(args) else {
        return;
    };
    let found = sessions::find_by_id(config, id);
    let chosen = match plan_resume(&found, target) {
        ResumePlan::Proceed => return,
        ResumePlan::Copy(i) => {
            println!();
            i18n.print(Msg::ResumeNotHere(id.to_string(), target.to_string()));
            println!("{}", describe(&found[i], i18n));
            println!();
            i18n.print(Msg::SessionCostNote);
            let q = Msg::ResumeCopyConfirm(found[i].account.clone(), target.to_string());
            if !confirm(i18n, q) {
                i18n.print(Msg::ResumeContinuingWithout);
                return;
            }
            i
        }
        ResumePlan::Choose(candidates) => {
            let here = found.iter().any(|s| s.account == target);
            println!();
            if here {
                i18n.print(Msg::ResumeSeveralCopies(id.to_string()));
            } else {
                i18n.print(Msg::ResumeNotHere(id.to_string(), target.to_string()));
            }
            match ask_which(&found, &candidates, i18n, Some(target)) {
                // Picking the target's own copy means "leave it alone".
                Some(i) if found[i].account == target => {
                    i18n.print(Msg::ResumeContinuingLocal(target.to_string()));
                    return;
                }
                Some(i) => i,
                // Distinct from picking the local copy on purpose: the
                // action is the same, but "I didn't catch that" and "keep
                // this one" must not look identical.
                None => {
                    if here {
                        i18n.print(Msg::ResumePickNoChoice(target.to_string()));
                    } else {
                        i18n.print(Msg::ResumeContinuingWithout);
                    }
                    return;
                }
            }
        }
    };

    match sessions::copy_into(&found[chosen], target_dir) {
        Ok(_) => i18n.print(Msg::ResumeCopied(
            found[chosen].account.clone(),
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

/// Decide whether the user needs to be asked anything. Pure, so every branch
/// is testable without touching a filesystem or a prompt.
///
/// The rule is "more than one copy in play means a question": a session that
/// exists both here and elsewhere is offered as a choice even when the local
/// copy is the newer one, because only the user knows which of the two
/// conversations they meant. A session no other account has is never worth a
/// prompt — that is claude's ordinary behaviour, error included.
fn plan_resume(found: &[SessionRef], target: &str) -> ResumePlan {
    let elsewhere: Vec<usize> = found
        .iter()
        .enumerate()
        .filter(|(_, s)| s.account != target)
        .map(|(i, _)| i)
        .collect();
    if elsewhere.is_empty() {
        return ResumePlan::Proceed;
    }

    let here = found.iter().any(|s| s.account == target);
    match (here, elsewhere.len()) {
        // Offer every copy, the local one included, so "keep this one" is a
        // choice rather than an escape.
        (true, _) => ResumePlan::Choose((0..found.len()).collect()),
        (false, 1) => ResumePlan::Copy(elsewhere[0]),
        (false, _) => ResumePlan::Choose(elsewhere),
    }
}

/// Config-file key controlling whether the generated `claude` wrapper runs
/// the `--resume` preflight.
pub const RESUME_HOOK_SETTING: &str = "resume_hook";

/// Whether the wrapper preflight should run. On unless explicitly turned off
/// — it costs nothing unless `--resume <id>` is actually present, and the
/// case it catches is otherwise invisible. `CLAUDE_ACC_NO_RESUME_HOOK` turns
/// it off for a single invocation without touching the config.
pub fn hook_enabled(config: &AppConfig) -> bool {
    if let Ok(v) = std::env::var("CLAUDE_ACC_NO_RESUME_HOOK")
        && !v.is_empty()
        && v != "0"
    {
        return false;
    }
    config.get_setting(RESUME_HOOK_SETTING).as_deref() != Some("off")
}

/// Entry point for the generated `claude` wrapper. `args` are the arguments
/// on their way to the real claude binary; the account is whatever
/// `CLAUDE_CONFIG_DIR` points at by the time the wrapper calls us.
///
/// Anything unexpected here is silently a no-op: this sits in front of every
/// `claude` launch, so nothing it does may prevent one.
pub fn preflight_hook(config: &AppConfig, i18n: &I18n, args: &[String]) -> i32 {
    if !hook_enabled(config) {
        return 0;
    }
    if let Some((label, dir)) = target_from_env(config) {
        preflight_resume(config, i18n, args, &label, &dir);
    }
    0
}

/// The account `CLAUDE_CONFIG_DIR` currently designates. An unset or empty
/// variable means the standard `~/.claude` account, exactly as it does for
/// claude itself.
fn target_from_env(config: &AppConfig) -> Option<(String, PathBuf)> {
    let dir = match std::env::var("CLAUDE_CONFIG_DIR") {
        Ok(v) if !v.trim().is_empty() => PathBuf::from(v.trim()),
        _ => identity::standard_token_dir()?,
    };
    if !dir.is_dir() {
        return None;
    }
    let label = label_for_dir(config, &dir);
    Some((label, dir))
}

/// The account label for a config directory, so prompts name it the same way
/// `claude-acc sessions` does. Falls back to the directory's own name for a
/// config dir this tool doesn't manage.
fn label_for_dir(config: &AppConfig, dir: &Path) -> String {
    let target = std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf());
    for (label, known) in sessions::account_config_dirs(config) {
        let known = std::fs::canonicalize(&known).unwrap_or(known);
        if known == target {
            return label;
        }
    }
    dir.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| dir.display().to_string())
}

/// The config directory for an account label, or `None` if there is no such
/// account. `"default"` means the standard `~/.claude`.
fn account_config_dir(config: &AppConfig, label: &str) -> Option<PathBuf> {
    if label == sessions::DEFAULT_LABEL {
        return identity::standard_token_dir().filter(|d| d.is_dir());
    }
    if !validate_name(label) || !config.account_exists(label) {
        return None;
    }
    Some(config.account_path(label))
}

/// Decide which copy to read from. Pure — the interactive prompt lives in the
/// caller so the decision itself stays testable.
///
/// Copies already sitting in the destination are never sources: copying an
/// account's own file onto itself is a no-op at best, and at worst hides the
/// fact that the id the user typed only exists where they're copying to.
fn pick_source(copies: &[SessionRef], to: &str, from: Option<&str>) -> Source {
    let candidates: Vec<usize> = copies
        .iter()
        .enumerate()
        .filter(|(_, s)| s.account != to)
        .map(|(i, _)| i)
        .collect();

    if let Some(want) = from {
        return match candidates.iter().find(|&&i| copies[i].account == want) {
            Some(&i) => Source::One(i),
            None => Source::UnknownFrom,
        };
    }

    match candidates.len() {
        0 => Source::AlreadyThere,
        1 => Source::One(candidates[0]),
        _ => Source::Ambiguous(candidates),
    }
}

/// Prompt for one of several candidate copies. Returns the chosen index into
/// `copies`, or `None` if the user cancelled or typed something unusable.
pub(super) fn ask_which(
    copies: &[SessionRef],
    candidates: &[usize],
    i18n: &I18n,
    current: Option<&str>,
) -> Option<usize> {
    for (n, &i) in candidates.iter().enumerate() {
        println!(
            "  [{}]{}{}",
            n + 1,
            describe(&copies[i], i18n),
            pick_markers(copies, candidates, i, current, i18n)
        );
    }
    println!();
    print!("{}", i18n.msg(Msg::SessionPickPrompt));
    io::stdout().flush().ok()?;

    let mut reply = String::new();
    io::stdin().read_line(&mut reply).ok()?;
    let n: usize = reply.trim().parse().ok()?;
    candidates.get(n.checked_sub(1)?).copied()
}

/// Trailing markers for one row of the pick list: which copy the current
/// account already holds, and which is the freshest. Without them the rows
/// are three columns of near-identical text, and the question the prompt is
/// asking — "which of these did you mean?" — has nothing visible to reason
/// from. The freshest marker is pointless with a single candidate, so it is
/// only drawn when there is a comparison to make.
fn pick_markers(
    copies: &[SessionRef],
    candidates: &[usize],
    i: usize,
    current: Option<&str>,
    i18n: &I18n,
) -> String {
    let mut parts: Vec<String> = Vec::new();
    if current.is_some() && Some(copies[i].account.as_str()) == current {
        parts.push(i18n.msg(Msg::ResumePickThisAccount));
    }
    let newest = candidates
        .iter()
        .copied()
        .max_by_key(|&c| copies[c].modified)
        .filter(|_| candidates.len() > 1);
    if newest == Some(i) {
        parts.push(i18n.msg(Msg::ResumePickNewest));
    }
    if parts.is_empty() {
        return String::new();
    }
    format!("  \u{2190} {}", parts.join(", "))
}

/// One line describing a copy: which account, how stale, how big. The age is
/// the point — when two accounts hold the same id, "which was touched last"
/// is what tells them apart.
pub(super) fn describe(s: &SessionRef, i18n: &I18n) -> String {
    let age = match s.modified {
        Some(epoch) => i18n.msg(Msg::RelativeTime(sessions::age_secs(epoch))),
        None => "?".to_string(),
    };
    format!(
        "  {:<12}  {:<12}  {}",
        s.account,
        age,
        sessions::human_size(s.size)
    )
}

pub(super) fn confirm(i18n: &I18n, question: Msg) -> bool {
    print!("{}", i18n.msg(question));
    if io::stdout().flush().is_err() {
        return false;
    }
    let mut reply = String::new();
    if io::stdin().read_line(&mut reply).is_err() {
        return false;
    }
    let reply = reply.trim().to_lowercase();
    reply.starts_with('y') || reply.starts_with('д')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::Lang;

    fn session(id: &str, account: &str, modified: u64) -> SessionRef {
        SessionRef {
            id: id.to_string(),
            account: account.to_string(),
            project: "proj".to_string(),
            path: PathBuf::from(format!("/tmp/{}.jsonl", id)),
            modified: Some(modified),
            size: 1024,
        }
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

    fn at(account: &str, modified: u64) -> SessionRef {
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
        assert_eq!(plan_resume(&[at("work", 100)], "work"), ResumePlan::Proceed);
    }

    #[test]
    fn offers_the_single_copy_held_elsewhere() {
        let copies = vec![at("personal", 100)];
        assert_eq!(plan_resume(&copies, "work"), ResumePlan::Copy(0));
    }

    #[test]
    fn asks_which_when_several_accounts_hold_it() {
        let copies = vec![at("personal", 100), at("other", 200)];
        assert_eq!(plan_resume(&copies, "work"), ResumePlan::Choose(vec![0, 1]));
    }

    #[test]
    fn a_copy_here_and_elsewhere_is_always_a_choice() {
        // Even when this account's copy is the newer one: only the user
        // knows which of the two conversations they meant.
        let newer_here = vec![at("work", 500), at("personal", 100)];
        assert_eq!(
            plan_resume(&newer_here, "work"),
            ResumePlan::Choose(vec![0, 1])
        );

        let newer_elsewhere = vec![at("work", 100), at("personal", 500)];
        assert_eq!(
            plan_resume(&newer_elsewhere, "work"),
            ResumePlan::Choose(vec![0, 1])
        );
    }

    #[test]
    fn the_local_copy_is_one_of_the_candidates() {
        // "Keep the one already here" has to be pickable, not only reachable
        // by cancelling.
        let copies = vec![at("work", 100), at("personal", 200), at("other", 900)];
        assert_eq!(
            plan_resume(&copies, "work"),
            ResumePlan::Choose(vec![0, 1, 2])
        );
    }

    #[test]
    fn label_for_dir_uses_the_managed_account_name() {
        let base = std::env::temp_dir().join(format!("cc-label-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let config = AppConfig {
            base_dir: base.clone(),
        };
        config.init().unwrap();
        let work = config.account_path("work");
        std::fs::create_dir_all(&work).unwrap();

        assert_eq!(label_for_dir(&config, &work), "work");
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn label_for_dir_falls_back_to_the_directory_name() {
        // A CLAUDE_CONFIG_DIR this tool doesn't manage still deserves a
        // readable name in the prompt rather than a blank.
        let base = std::env::temp_dir().join(format!("cc-label-alien-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let config = AppConfig {
            base_dir: base.clone(),
        };
        config.init().unwrap();
        let alien = base.join("somewhere-else");
        std::fs::create_dir_all(&alien).unwrap();

        assert_eq!(label_for_dir(&config, &alien), "somewhere-else");
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn single_copy_elsewhere_needs_no_question() {
        let copies = vec![session("s", "work", 100)];
        assert_eq!(pick_source(&copies, "personal", None), Source::One(0));
    }

    #[test]
    fn copy_already_in_destination_is_not_a_source() {
        let copies = vec![session("s", "work", 100)];
        assert_eq!(pick_source(&copies, "work", None), Source::AlreadyThere);
    }

    #[test]
    fn two_copies_elsewhere_are_ambiguous() {
        let copies = vec![session("s", "work", 300), session("s", "personal", 100)];
        assert_eq!(
            pick_source(&copies, "default", None),
            Source::Ambiguous(vec![0, 1])
        );
    }

    #[test]
    fn destination_copy_is_excluded_from_the_ambiguity() {
        // Three copies, one of them in the destination: only the other two
        // are real choices, so there is exactly one left to pick.
        let copies = vec![
            session("s", "default", 300),
            session("s", "work", 200),
            session("s", "personal", 100),
        ];
        assert_eq!(
            pick_source(&copies, "work", Some("personal")),
            Source::One(2)
        );
        assert_eq!(
            pick_source(&copies, "work", None),
            Source::Ambiguous(vec![0, 2])
        );
    }

    #[test]
    fn explicit_from_selects_that_account() {
        let copies = vec![session("s", "work", 300), session("s", "personal", 100)];
        assert_eq!(
            pick_source(&copies, "default", Some("personal")),
            Source::One(1)
        );
    }

    #[test]
    fn from_naming_the_destination_is_not_a_source() {
        let copies = vec![session("s", "work", 300), session("s", "personal", 100)];
        assert_eq!(
            pick_source(&copies, "work", Some("work")),
            Source::UnknownFrom
        );
    }

    #[test]
    fn from_naming_an_account_without_a_copy_is_rejected() {
        let copies = vec![session("s", "work", 100)];
        assert_eq!(
            pick_source(&copies, "default", Some("nowhere")),
            Source::UnknownFrom
        );
    }

    #[test]
    fn describe_shows_account_age_and_size() {
        let i18n = I18n { lang: Lang::En };
        let line = describe(&session("s", "work", 0), &i18n);
        assert!(line.contains("work"), "{}", line);
        assert!(line.contains("1 KB"), "{}", line);
    }
}
