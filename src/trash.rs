//! Moving a directory to the desktop trash instead of deleting it outright.
//!
//! Getting a removal wrong should cost a drag out of the Trash, not a restore
//! from backup. An account directory holds transcripts, settings and plugins
//! that exist nowhere else, and `remove` used to unlink all of it with no way
//! back.
//!
//! This is a move, never a copy: if the trash lives on another filesystem the
//! rename fails and we say so rather than duplicating gigabytes to "save"
//! them. The caller decides what to do then — `remove` falls back to deleting
//! outright, and says which of the two happened.
//!
//! Where things land:
//!
//! | | |
//! |---|---|
//! | macOS | `~/.Trash/<name>` |
//! | Linux | `$XDG_DATA_HOME/Trash` (default `~/.local/share/Trash`), per the freedesktop spec: the directory under `files/`, a `.trashinfo` beside it under `info/` |
//! | Windows | unsupported — the Recycle Bin needs a Win32 shell call this tool has no binding for |
//!
//! The `.trashinfo` file is what makes a Linux file manager offer "Restore".
//! Without it the directory still sits in the trash folder, but nothing knows
//! where it came from.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Move `path` into the desktop trash. Returns where it landed.
///
/// `ErrorKind::Unsupported` means this platform has no trash we can reach, and
/// is the caller's cue to fall back — it is not a failure to report as one.
pub fn trash_dir(path: &Path) -> io::Result<PathBuf> {
    let stem = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "no file name to trash"))?;
    trash_dir_into(&trash_root()?, path, stem)
}

/// The move itself, with the trash location passed in so it can be tested
/// against a scratch directory rather than the real one.
fn trash_dir_into(root: &TrashRoot, path: &Path, stem: &str) -> io::Result<PathBuf> {
    fs::create_dir_all(&root.files)?;
    if let Some(info) = &root.info {
        fs::create_dir_all(info)?;
    }

    let name = pick_free_name(&root.files, stem);
    let dest = root.files.join(&name);

    // Rename, never copy. Across filesystems this fails with EXDEV, and the
    // caller hears about it — spending 10 GB of writes to avoid deleting
    // 10 GB would be a worse answer than saying it cannot be done.
    fs::rename(path, &dest)?;

    // Best effort: the move is what matters, and a trash entry missing its
    // sidecar is still recoverable by hand.
    if let Some(info) = &root.info {
        let body = trashinfo(path, &iso_local_seconds(now_secs()));
        let _ = fs::write(info.join(format!("{}.trashinfo", name)), body);
    }
    Ok(dest)
}

/// Where the trash lives, and whether it keeps `.trashinfo` sidecars.
struct TrashRoot {
    files: PathBuf,
    info: Option<PathBuf>,
}

fn trash_root() -> io::Result<TrashRoot> {
    let unsupported = || {
        io::Error::new(
            io::ErrorKind::Unsupported,
            "no desktop trash on this platform",
        )
    };
    let home = dirs::home_dir().ok_or_else(unsupported)?;

    if cfg!(target_os = "macos") {
        // The Finder trash is a plain directory; moving into it needs no
        // automation permission, which an AppleScript "delete" would prompt
        // for. The cost is no "Put Back" — the item is recoverable, but the
        // Finder does not know where it came from.
        return Ok(TrashRoot {
            files: home.join(".Trash"),
            info: None,
        });
    }
    if cfg!(target_os = "linux") {
        let base = std::env::var_os("XDG_DATA_HOME")
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".local/share"));
        let trash = base.join("Trash");
        return Ok(TrashRoot {
            files: trash.join("files"),
            info: Some(trash.join("info")),
        });
    }
    Err(unsupported())
}

/// A name not already taken in `dir`.
///
/// Both trashes are flat, shared with everything else the user has thrown
/// away, so a collision is ordinary rather than exceptional — removing an
/// account called `work` twice is exactly the case.
fn pick_free_name(dir: &Path, stem: &str) -> String {
    (0..)
        .map(|n| nth_candidate(stem, n))
        .find(|c| !dir.join(c).exists())
        // `(0..)` is infinite, so `find` only ends by succeeding.
        .unwrap_or_else(|| nth_candidate(stem, 0))
}

/// `work`, `work 2`, `work 3`, … — the Finder's own spelling for this.
fn nth_candidate(stem: &str, n: u64) -> String {
    if n == 0 {
        stem.to_string()
    } else {
        format!("{} {}", stem, n + 1)
    }
}

/// The freedesktop sidecar: where it came from, and when it went.
///
/// `Path` is percent-encoded per the spec — a `#` or a space in a real path
/// would otherwise make the entry unparseable to the file manager reading it.
fn trashinfo(original: &Path, deleted_at: &str) -> String {
    format!(
        "[Trash Info]\nPath={}\nDeletionDate={}\n",
        percent_encode(&original.to_string_lossy()),
        deleted_at
    )
}

/// Percent-encode everything outside the unreserved set, leaving `/` alone so
/// the path stays readable as a path.
fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Unix seconds as `YYYY-MM-DDThh:mm:ss`, the shape the spec asks for.
///
/// Written out rather than pulled from a date crate: this is the only place
/// the tool formats a date, and the binary stays dependency-light on purpose.
/// The value is UTC — the spec says local time, and a file manager shows this
/// only as "deleted on", so an offset is not worth a timezone database.
fn iso_local_seconds(epoch: i64) -> String {
    let days = epoch.div_euclid(86_400);
    let secs = epoch.rem_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
        y,
        m,
        d,
        secs / 3600,
        (secs % 3600) / 60,
        secs % 60
    )
}

/// Days since the epoch to a calendar date (Howard Hinnant's `civil_from_days`).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("cc-trash-{}-{}", name, std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn root_at(base: &Path, with_info: bool) -> TrashRoot {
        TrashRoot {
            files: base.join("files"),
            info: with_info.then(|| base.join("info")),
        }
    }

    #[test]
    fn a_directory_is_moved_not_copied() {
        let base = scratch("move");
        let src = base.join("work");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("marker"), "contents").unwrap();

        let dest = trash_dir_into(&root_at(&base, false), &src, "work").unwrap();

        assert!(!src.exists(), "the original must be gone");
        assert_eq!(fs::read_to_string(dest.join("marker")).unwrap(), "contents");
        let _ = fs::remove_dir_all(&base);
    }

    // Removing an account called `work` twice is the ordinary case, not an
    // exceptional one: the trash is flat and shared with everything else the
    // user has thrown away.
    #[test]
    fn a_second_entry_with_the_same_name_does_not_overwrite_the_first() {
        let base = scratch("collide");
        let root = root_at(&base, false);

        for body in ["first", "second"] {
            let src = base.join("work");
            fs::create_dir_all(&src).unwrap();
            fs::write(src.join("marker"), body).unwrap();
            trash_dir_into(&root, &src, "work").unwrap();
        }

        assert_eq!(
            fs::read_to_string(root.files.join("work/marker")).unwrap(),
            "first"
        );
        assert_eq!(
            fs::read_to_string(root.files.join("work 2/marker")).unwrap(),
            "second"
        );
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn candidates_are_numbered_the_way_the_finder_numbers_them() {
        assert_eq!(nth_candidate("work", 0), "work");
        assert_eq!(nth_candidate("work", 1), "work 2");
        assert_eq!(nth_candidate("work", 9), "work 10");
    }

    // Without the sidecar the entry is still in the trash, but no file
    // manager can offer to put it back.
    #[test]
    fn a_trashinfo_sidecar_is_written_when_the_platform_wants_one() {
        let base = scratch("info");
        let src = base.join("work");
        fs::create_dir_all(&src).unwrap();

        trash_dir_into(&root_at(&base, true), &src, "work").unwrap();

        let body = fs::read_to_string(base.join("info/work.trashinfo")).unwrap();
        assert!(body.starts_with("[Trash Info]\n"), "{body}");
        assert!(body.contains("Path=/"), "{body}");
        assert!(body.contains("DeletionDate=20"), "{body}");
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn no_sidecar_is_written_where_the_platform_has_no_use_for_one() {
        let base = scratch("noinfo");
        let src = base.join("work");
        fs::create_dir_all(&src).unwrap();

        trash_dir_into(&root_at(&base, false), &src, "work").unwrap();

        assert!(!base.join("info").exists());
        let _ = fs::remove_dir_all(&base);
    }

    // A space or a `#` in the path would make the sidecar unparseable to the
    // file manager reading it.
    #[test]
    fn the_original_path_is_percent_encoded() {
        let body = trashinfo(Path::new("/home/a b/#c"), "2026-09-26T08:30:00");
        assert!(body.contains("Path=/home/a%20b/%23c"), "{body}");
        assert!(!body.contains("Path=/home/a b"), "{body}");
    }

    #[test]
    fn unreserved_characters_survive_encoding() {
        assert_eq!(percent_encode("/a-b_c.d~e/9Z"), "/a-b_c.d~e/9Z");
    }

    #[test]
    fn the_deletion_date_has_the_shape_the_spec_asks_for() {
        assert_eq!(iso_local_seconds(0), "1970-01-01T00:00:00");
        assert_eq!(iso_local_seconds(1_774_512_000), "2026-03-26T08:00:00");
        // A leap day, where a naive day-count conversion goes wrong.
        assert_eq!(iso_local_seconds(1_709_208_000), "2024-02-29T12:00:00");
    }

    #[test]
    fn a_path_with_no_file_name_is_refused_rather_than_guessed_at() {
        assert_eq!(
            trash_dir(Path::new("/")).unwrap_err().kind(),
            io::ErrorKind::InvalidInput
        );
    }
}
