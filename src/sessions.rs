//! Locating Claude Code session transcripts across accounts.
//!
//! Claude Code writes one session as
//! `<CLAUDE_CONFIG_DIR>/projects/<slug>/<uuid>.jsonl`, plus an optional
//! sidecar directory `<slug>/<uuid>/` holding subagent transcripts. Because
//! this tool gives every account its own `CLAUDE_CONFIG_DIR`, each account
//! also gets its own `projects/` tree — which is exactly why a session
//! started under one account is invisible to `claude --resume` under another.
//!
//! The transcript format itself carries no account identity (no email, no
//! user id, no organization uuid — those live in `.claude.json`, which we
//! never touch), so a transcript is portable between accounts as a plain
//! file copy.

use crate::config::AppConfig;
use crate::identity;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Label used for the standard `~/.claude` account, matching the `default`
/// name the other commands accept.
pub const DEFAULT_LABEL: &str = "default";

/// One session transcript belonging to one account.
#[derive(Debug, Clone, PartialEq)]
pub struct SessionRef {
    /// Session uuid — the `<id>` in `claude --resume <id>`.
    pub id: String,
    /// Account label this copy lives in ("default" for the standard account).
    pub account: String,
    /// The `projects/` subdirectory name (the slugified working directory).
    pub project: String,
    /// Path to the `.jsonl` transcript itself.
    pub path: PathBuf,
    /// Last-modified time, Unix epoch seconds. `None` if unreadable.
    pub modified: Option<u64>,
    /// Transcript size in bytes.
    pub size: u64,
}

impl SessionRef {
    /// The sidecar directory holding this session's subagent transcripts.
    /// Often absent — a session that never spawned a subagent has none.
    pub fn sidecar_dir(&self) -> PathBuf {
        self.path.with_extension("")
    }
}

/// What a copy actually moved, for reporting back to the user.
#[derive(Debug, Clone, PartialEq)]
pub struct CopyReport {
    /// Where the transcript landed.
    pub dest: PathBuf,
    /// Transcript bytes written.
    pub bytes: u64,
    /// Subagent transcripts copied alongside it (0 when there was no sidecar).
    pub subagents: usize,
}

/// Slugify a working directory the way Claude Code names its `projects/`
/// subdirectories: every non-alphanumeric character becomes `-`.
///
/// Inferred from the on-disk layout rather than documented, and verified
/// against a sample of real project directories (paths containing `/`, `.`
/// and `-` all round-trip). Callers that must not silently show an empty
/// list should offer an "all projects" fallback.
pub fn project_slug(cwd: &Path) -> String {
    cwd.to_string_lossy()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

/// The `projects/` tree inside one account's config directory.
pub fn projects_dir(config_dir: &Path) -> PathBuf {
    config_dir.join("projects")
}

/// `(label, config_dir)` for every account this tool knows about: each
/// managed account, plus the standard `~/.claude` account as "default".
/// Accounts whose directory doesn't exist are skipped.
pub fn account_config_dirs(config: &AppConfig) -> Vec<(String, PathBuf)> {
    let mut dirs: Vec<(String, PathBuf)> = config
        .list_accounts()
        .unwrap_or_default()
        .into_iter()
        .map(|acc| {
            let dir = config.account_path(&acc);
            (acc, dir)
        })
        .collect();
    if let Some(dir) = identity::standard_token_dir()
        && dir.is_dir()
    {
        dirs.push((DEFAULT_LABEL.to_string(), dir));
    }
    dirs
}

/// Every session in one account, newest first. `slug` restricts the search to
/// a single project; `None` walks every project in that account.
pub fn list_in(config_dir: &Path, account: &str, slug: Option<&str>) -> Vec<SessionRef> {
    let root = projects_dir(config_dir);
    let project_dirs: Vec<PathBuf> = match slug {
        Some(s) => vec![root.join(s)],
        None => match fs::read_dir(&root) {
            Ok(entries) => entries
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
                .map(|e| e.path())
                .collect(),
            Err(_) => return Vec::new(),
        },
    };

    let mut found = Vec::new();
    for dir in project_dirs {
        let project = match dir.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                continue;
            }
            let Some(id) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            let meta = entry.metadata().ok();
            found.push(SessionRef {
                id: id.to_string(),
                account: account.to_string(),
                project: project.clone(),
                path: path.clone(),
                modified: meta.as_ref().and_then(modified_epoch),
                size: meta.as_ref().map(|m| m.len()).unwrap_or(0),
            });
        }
    }
    sort_newest_first(&mut found);
    found
}

/// Every session across every account, newest first.
pub fn list_all(config: &AppConfig, slug: Option<&str>) -> Vec<SessionRef> {
    let mut found: Vec<SessionRef> = account_config_dirs(config)
        .iter()
        .flat_map(|(label, dir)| list_in(dir, label, slug))
        .collect();
    sort_newest_first(&mut found);
    found
}

/// Every copy of one session id, across every account, newest first. More
/// than one result means the transcript has been copied around and the copies
/// have since drifted apart.
pub fn find_by_id(config: &AppConfig, id: &str) -> Vec<SessionRef> {
    let mut found: Vec<SessionRef> = list_all(config, None)
        .into_iter()
        .filter(|s| s.id == id)
        .collect();
    sort_newest_first(&mut found);
    found
}

/// Where `src` would land inside `dest_config_dir`. The project slug is
/// carried over from the source rather than re-derived, so this never has to
/// guess how Claude Code named the directory.
pub fn destination_path(src: &SessionRef, dest_config_dir: &Path) -> PathBuf {
    projects_dir(dest_config_dir)
        .join(&src.project)
        .join(format!("{}.jsonl", src.id))
}

/// Copy `src`'s transcript (and its subagent sidecar, if any) into
/// `dest_config_dir`, so `claude --resume <id>` can see it from that account.
///
/// The transcript is staged next to its destination and renamed into place,
/// so an interrupted copy can't leave a half-written transcript where a
/// complete one is expected. The sidecar is copied afterwards: losing it
/// degrades subagent history but leaves the conversation itself resumable.
///
/// Whether the copy inherits the source's modification time is left to the
/// platform (macOS's `copyfile` preserves it, Linux's copy doesn't). Either
/// reading of "which copy is newest" is defensible — the last conversation
/// activity, or the last local write — and both stay accurate afterwards,
/// since resuming a session rewrites its transcript.
pub fn copy_into(src: &SessionRef, dest_config_dir: &Path) -> std::io::Result<CopyReport> {
    let dest = destination_path(src, dest_config_dir);
    let parent = dest
        .parent()
        .ok_or_else(|| std::io::Error::other("destination has no parent directory"))?;
    fs::create_dir_all(parent)?;

    let staged = dest.with_extension("jsonl.part");
    // A leftover from an earlier interrupted copy must not be appended to.
    let _ = fs::remove_file(&staged);
    let bytes = fs::copy(&src.path, &staged)?;
    fs::rename(&staged, &dest)?;

    let sidecar_src = src.sidecar_dir();
    let subagents = if sidecar_src.is_dir() {
        copy_dir(&sidecar_src, &dest.with_extension(""))?
    } else {
        0
    };

    Ok(CopyReport {
        dest,
        bytes,
        subagents,
    })
}

/// Recursively copy a directory of plain files, returning the file count.
/// The sidecar holds only `.jsonl` / `.meta.json` files written by Claude
/// Code, so there are no symlinks to preserve here.
fn copy_dir(src: &Path, dst: &Path) -> std::io::Result<usize> {
    fs::create_dir_all(dst)?;
    let mut copied = 0;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copied += copy_dir(&from, &to)?;
        } else {
            fs::copy(&from, &to)?;
            copied += 1;
        }
    }
    Ok(copied)
}

/// Sort newest first, with unreadable timestamps last and a stable
/// account/id tiebreak so identical mtimes don't reorder between runs.
pub fn sort_newest_first(sessions: &mut [SessionRef]) {
    sessions.sort_by(|a, b| {
        b.modified
            .cmp(&a.modified)
            .then_with(|| a.account.cmp(&b.account))
            .then_with(|| a.id.cmp(&b.id))
    });
}

/// Ids that appear in more than one account — the sessions where "which copy
/// do you mean?" is a real question.
pub fn duplicated_ids(sessions: &[SessionRef]) -> Vec<String> {
    let mut ids: Vec<&str> = sessions.iter().map(|s| s.id.as_str()).collect();
    ids.sort_unstable();
    let mut dups: Vec<String> = Vec::new();
    for pair in ids.windows(2) {
        if pair[0] == pair[1] && dups.last().map(String::as_str) != Some(pair[0]) {
            dups.push(pair[0].to_string());
        }
    }
    dups
}

fn modified_epoch(meta: &fs::Metadata) -> Option<u64> {
    meta.modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs())
}

/// Seconds between `epoch` and now. Returns 0 for timestamps in the future
/// (clock skew), so callers can render "just now" rather than a negative age.
pub fn age_secs(epoch: u64) -> u64 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    now.saturating_sub(epoch)
}

/// Byte count as a short human string: `"918 B"`, `"61 KB"`, `"9.2 MB"`.
pub fn human_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    if bytes < KB {
        format!("{} B", bytes)
    } else if bytes < MB {
        format!("{} KB", bytes / KB)
    } else if bytes < GB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_slug_replaces_separators() {
        assert_eq!(
            project_slug(Path::new("/Users/me/Documents/my-repo")),
            "-Users-me-Documents-my-repo"
        );
    }

    #[test]
    fn project_slug_replaces_dots_and_underscores() {
        // A git worktree under `.worktrees/` is the common real case.
        assert_eq!(
            project_slug(Path::new("/home/me/rpg-game/.worktrees/ai")),
            "-home-me-rpg-game--worktrees-ai"
        );
        assert_eq!(project_slug(Path::new("/tmp/a_b")), "-tmp-a-b");
    }

    fn session(id: &str, account: &str, modified: u64) -> SessionRef {
        SessionRef {
            id: id.to_string(),
            account: account.to_string(),
            project: "proj".to_string(),
            path: PathBuf::from(format!("/tmp/projects/proj/{}.jsonl", id)),
            modified: Some(modified),
            size: 0,
        }
    }

    #[test]
    fn sort_puts_newest_first() {
        let mut v = vec![
            session("a", "work", 100),
            session("b", "work", 300),
            session("c", "work", 200),
        ];
        sort_newest_first(&mut v);
        let ids: Vec<&str> = v.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(ids, vec!["b", "c", "a"]);
    }

    #[test]
    fn sort_puts_unknown_timestamps_last() {
        let mut unknown = session("x", "work", 0);
        unknown.modified = None;
        let mut v = vec![unknown, session("a", "work", 100)];
        sort_newest_first(&mut v);
        assert_eq!(v[0].id, "a");
        assert_eq!(v[1].id, "x");
    }

    #[test]
    fn duplicated_ids_finds_ids_present_in_two_accounts() {
        let v = vec![
            session("same", "work", 100),
            session("same", "default", 200),
            session("only-here", "work", 300),
        ];
        assert_eq!(duplicated_ids(&v), vec!["same".to_string()]);
    }

    #[test]
    fn duplicated_ids_reports_each_id_once() {
        let v = vec![
            session("same", "work", 100),
            session("same", "default", 200),
            session("same", "other", 300),
        ];
        assert_eq!(duplicated_ids(&v), vec!["same".to_string()]);
    }

    #[test]
    fn duplicated_ids_empty_when_all_unique() {
        let v = vec![session("a", "work", 100), session("b", "default", 200)];
        assert!(duplicated_ids(&v).is_empty());
    }

    /// A source account holding one session (with an optional subagent
    /// sidecar), plus an empty destination config dir.
    fn fixture(tag: &str, with_sidecar: bool) -> (PathBuf, SessionRef, PathBuf) {
        let base = std::env::temp_dir().join(format!("cc-sessions-{}-{}", tag, std::process::id()));
        let _ = fs::remove_dir_all(&base);
        let src_project = base.join("src/projects/-tmp-proj");
        fs::create_dir_all(&src_project).unwrap();
        let path = src_project.join("sess-1.jsonl");
        fs::write(&path, b"{\"type\":\"user\"}\n").unwrap();
        if with_sidecar {
            let side = src_project.join("sess-1/subagents");
            fs::create_dir_all(&side).unwrap();
            fs::write(side.join("agent-a.jsonl"), b"{}\n").unwrap();
            fs::write(side.join("agent-a.meta.json"), b"{}").unwrap();
        }
        let src = SessionRef {
            id: "sess-1".to_string(),
            account: "work".to_string(),
            project: "-tmp-proj".to_string(),
            path,
            modified: Some(1),
            size: 16,
        };
        (base.clone(), src, base.join("dest"))
    }

    #[test]
    fn destination_keeps_the_source_project_slug() {
        let (base, src, dest_cfg) = fixture("dest-path", false);
        assert_eq!(
            destination_path(&src, &dest_cfg),
            dest_cfg.join("projects/-tmp-proj/sess-1.jsonl")
        );
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn copy_into_creates_the_transcript_in_the_destination_account() {
        let (base, src, dest_cfg) = fixture("copy-plain", false);
        let report = copy_into(&src, &dest_cfg).unwrap();

        assert!(report.dest.is_file());
        assert_eq!(fs::read(&report.dest).unwrap(), b"{\"type\":\"user\"}\n");
        assert_eq!(report.bytes, 16);
        assert_eq!(report.subagents, 0);
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn copy_into_brings_the_subagent_sidecar_along() {
        let (base, src, dest_cfg) = fixture("copy-sidecar", true);
        let report = copy_into(&src, &dest_cfg).unwrap();

        assert_eq!(report.subagents, 2);
        let side = dest_cfg.join("projects/-tmp-proj/sess-1/subagents");
        assert!(side.join("agent-a.jsonl").is_file());
        assert!(side.join("agent-a.meta.json").is_file());
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn copy_into_leaves_no_staging_file_behind() {
        let (base, src, dest_cfg) = fixture("copy-staging", false);
        copy_into(&src, &dest_cfg).unwrap();

        let leftovers: Vec<String> = fs::read_dir(dest_cfg.join("projects/-tmp-proj"))
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .filter(|n| n.ends_with(".part"))
            .collect();
        assert!(leftovers.is_empty(), "left behind: {:?}", leftovers);
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn copy_into_overwrites_an_existing_copy_rather_than_appending() {
        let (base, src, dest_cfg) = fixture("copy-overwrite", false);
        let dest = destination_path(&src, &dest_cfg);
        fs::create_dir_all(dest.parent().unwrap()).unwrap();
        fs::write(&dest, b"stale content that is clearly longer\n").unwrap();

        copy_into(&src, &dest_cfg).unwrap();

        assert_eq!(fs::read(&dest).unwrap(), b"{\"type\":\"user\"}\n");
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn copy_into_ignores_a_stale_staging_file_from_an_interrupted_run() {
        let (base, src, dest_cfg) = fixture("copy-stale-part", false);
        let dest = destination_path(&src, &dest_cfg);
        fs::create_dir_all(dest.parent().unwrap()).unwrap();
        fs::write(dest.with_extension("jsonl.part"), b"half-written garbage").unwrap();

        let report = copy_into(&src, &dest_cfg).unwrap();

        assert_eq!(fs::read(&report.dest).unwrap(), b"{\"type\":\"user\"}\n");
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn list_in_finds_the_transcript_but_not_the_sidecar() {
        let (base, _src, _dest) = fixture("list-in", true);
        let found = list_in(&base.join("src"), "work", Some("-tmp-proj"));

        assert_eq!(found.len(), 1, "found: {:?}", found);
        assert_eq!(found[0].id, "sess-1");
        assert_eq!(found[0].account, "work");
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn human_size_units() {
        assert_eq!(human_size(918), "918 B");
        assert_eq!(human_size(62445), "60 KB");
        assert_eq!(human_size(9_685_413), "9.2 MB");
        assert_eq!(human_size(3_221_225_472), "3.0 GB");
    }

    #[test]
    fn age_secs_clamps_future_timestamps() {
        let future = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + 10_000;
        assert_eq!(age_secs(future), 0);
    }
}
