use crate::config::AppConfig;
use crate::i18n::{I18n, Msg};
use crate::sessions::{self, SessionRef};

pub fn run(config: &AppConfig, i18n: &I18n, all: bool) -> i32 {
    let cwd = std::env::current_dir().ok();
    let slug = if all {
        None
    } else {
        cwd.as_deref().map(sessions::project_slug)
    };

    let found = sessions::list_all(config, slug.as_deref());
    if found.is_empty() {
        match (all, &cwd) {
            (false, Some(dir)) => {
                i18n.print(Msg::SessionsEmptyHere(dir.display().to_string()));
            }
            _ => i18n.print(Msg::SessionsEmpty),
        }
        return 0;
    }

    match (all, &cwd) {
        (false, Some(dir)) => i18n.print(Msg::SessionsHeader(dir.display().to_string())),
        _ => i18n.print(Msg::SessionsHeaderAll),
    }
    println!();

    let dups = sessions::duplicated_ids(&found);
    let rows = layout(&found, &dups, i18n);
    for row in &rows {
        println!("{}", row);
    }

    if !dups.is_empty() {
        println!();
        i18n.print(Msg::SessionsDuplicateNote);
    }
    println!();
    i18n.print(Msg::SessionsHintResume);
    0
}

/// Render one aligned row per session. Sessions whose id lives in more than
/// one account are marked, and the freshest copy of such an id is flagged —
/// that is the whole point of the listing: seeing at a glance which account
/// holds the version you actually want.
fn layout(found: &[SessionRef], dups: &[String], i18n: &I18n) -> Vec<String> {
    let acc_width = found.iter().map(|s| s.account.len()).max().unwrap_or(0);
    let age_cells: Vec<String> = found.iter().map(|s| age_cell(s, i18n)).collect();
    let age_width = age_cells
        .iter()
        .map(|c| c.chars().count())
        .max()
        .unwrap_or(0);
    let newest = newest_per_duplicated_id(found, dups);

    found
        .iter()
        .zip(&age_cells)
        .map(|(s, age)| {
            let pad = " ".repeat(age_width - age.chars().count());
            let marker = if newest.contains(&(s.id.as_str(), s.account.as_str())) {
                format!("  {}", i18n.msg(Msg::SessionsNewestCopy))
            } else {
                String::new()
            };
            format!(
                "  {}  {:<acc$}  {}{}  {:>9}{}",
                s.id,
                s.account,
                age,
                pad,
                sessions::human_size(s.size),
                marker,
                acc = acc_width,
            )
        })
        .collect()
}

/// `(id, account)` of the freshest copy of every id that exists in more than
/// one account. Ids present only once are never marked — there is nothing to
/// choose between.
fn newest_per_duplicated_id<'a>(
    found: &'a [SessionRef],
    dups: &[String],
) -> Vec<(&'a str, &'a str)> {
    dups.iter()
        .filter_map(|id| {
            found
                .iter()
                .filter(|s| &s.id == id)
                .max_by_key(|s| s.modified)
                .map(|s| (s.id.as_str(), s.account.as_str()))
        })
        .collect()
}

fn age_cell(s: &SessionRef, i18n: &I18n) -> String {
    match s.modified {
        Some(epoch) => i18n.msg(Msg::RelativeTime(sessions::age_secs(epoch))),
        None => "?".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::Lang;
    use std::path::PathBuf;

    fn i18n() -> I18n {
        I18n { lang: Lang::En }
    }

    fn session(id: &str, account: &str, modified: u64, size: u64) -> SessionRef {
        SessionRef {
            id: id.to_string(),
            account: account.to_string(),
            project: "proj".to_string(),
            path: PathBuf::from(format!("/tmp/{}.jsonl", id)),
            modified: Some(modified),
            size,
        }
    }

    #[test]
    fn newest_copy_is_the_one_with_the_latest_mtime() {
        let found = vec![
            session("dup", "work", 100, 10),
            session("dup", "default", 900, 10),
        ];
        let newest = newest_per_duplicated_id(&found, &["dup".to_string()]);
        assert_eq!(newest, vec![("dup", "default")]);
    }

    #[test]
    fn unique_ids_are_never_marked_newest() {
        let found = vec![session("only", "work", 100, 10)];
        assert!(newest_per_duplicated_id(&found, &[]).is_empty());
    }

    #[test]
    fn rows_mark_only_the_freshest_duplicate() {
        let found = vec![
            session("dup", "default", 900, 10),
            session("dup", "work", 100, 10),
        ];
        let rows = layout(&found, &["dup".to_string()], &i18n());
        assert!(rows[0].contains("newest"), "{}", rows[0]);
        assert!(!rows[1].contains("newest"), "{}", rows[1]);
    }

    #[test]
    fn rows_align_account_column_across_differing_name_lengths() {
        let found = vec![
            session("a", "work", 900, 10),
            session("b", "a-very-long-account", 800, 10),
        ];
        let rows = layout(&found, &[], &i18n());
        // Both rows put the age cell at the same column.
        let at = |r: &String| r.find("ago").expect("age cell present");
        assert_eq!(at(&rows[0]), at(&rows[1]));
    }

    #[test]
    fn rows_show_size_and_account() {
        let found = vec![session("abc", "work", 900, 2048)];
        let rows = layout(&found, &[], &i18n());
        assert!(rows[0].contains("abc"));
        assert!(rows[0].contains("work"));
        assert!(rows[0].contains("2 KB"));
    }
}
