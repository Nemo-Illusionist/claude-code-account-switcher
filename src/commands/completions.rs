use crate::config::AppConfig;
use crate::sessions;

/// Data for the shell completion functions, one item per line. Everything
/// here is read by `$(...)` in a completion hook, so it stays plain and
/// silent: an unknown `what` prints nothing rather than an error, since a
/// stray message would end up offered as a completion candidate.
pub fn run(config: &AppConfig, what: &str) {
    match what {
        "accounts" => {
            for acc in config.list_accounts().unwrap_or_default() {
                println!("{}", acc);
            }
        }
        "sessions" => {
            for id in session_ids(config) {
                println!("{}", id);
            }
        }
        _ => {}
    }
}

/// Session ids for the current directory, newest first, each listed once
/// however many accounts hold a copy.
///
/// Scoped to the current project on purpose: a full listing runs to hundreds
/// of uuids across every project ever opened, which is not a menu anyone can
/// pick from. The ids you might plausibly want to resume or copy right now
/// are the ones belonging to where you are.
fn session_ids(config: &AppConfig) -> Vec<String> {
    let Ok(cwd) = std::env::current_dir() else {
        return Vec::new();
    };
    let slug = sessions::project_slug(&cwd);
    dedup_keeping_order(
        sessions::list_all(config, Some(&slug))
            .into_iter()
            .map(|s| s.id),
    )
}

/// Deduplicate while preserving first-seen order — the input is already
/// sorted newest first, and sorting again by id would throw that away.
fn dedup_keeping_order(items: impl Iterator<Item = String>) -> Vec<String> {
    let mut seen: Vec<String> = Vec::new();
    for item in items {
        if !seen.contains(&item) {
            seen.push(item);
        }
    }
    seen
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(v: &[&str]) -> Vec<String> {
        dedup_keeping_order(v.iter().map(|s| s.to_string()))
    }

    #[test]
    fn dedup_keeps_the_first_occurrence_of_each_id() {
        // Same session in two accounts must be offered once.
        assert_eq!(ids(&["a", "b", "a"]), vec!["a", "b"]);
    }

    #[test]
    fn dedup_preserves_newest_first_order() {
        assert_eq!(
            ids(&["newest", "older", "oldest"]),
            vec!["newest", "older", "oldest"]
        );
    }

    #[test]
    fn dedup_of_nothing_is_nothing() {
        assert!(ids(&[]).is_empty());
    }
}
