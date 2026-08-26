use crate::config::{AppConfig, validate_name};
use crate::i18n::{I18n, Msg};
use crate::identity;
use crate::sessions::{self, SessionRef};
use std::io::{self, Write};
use std::path::PathBuf;

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
            match ask_which(&copies, &candidates, i18n) {
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
fn ask_which(copies: &[SessionRef], candidates: &[usize], i18n: &I18n) -> Option<usize> {
    println!();
    i18n.print(Msg::SessionPickSource);
    for (n, &i) in candidates.iter().enumerate() {
        println!("  [{}]{}", n + 1, describe(&copies[i], i18n));
    }
    println!();
    print!("{}", i18n.msg(Msg::SessionPickPrompt));
    io::stdout().flush().ok()?;

    let mut reply = String::new();
    io::stdin().read_line(&mut reply).ok()?;
    let n: usize = reply.trim().parse().ok()?;
    candidates.get(n.checked_sub(1)?).copied()
}

/// One line describing a copy: which account, how stale, how big. The age is
/// the point — when two accounts hold the same id, "which was touched last"
/// is what tells them apart.
fn describe(s: &SessionRef, i18n: &I18n) -> String {
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

fn confirm(i18n: &I18n, question: Msg) -> bool {
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
