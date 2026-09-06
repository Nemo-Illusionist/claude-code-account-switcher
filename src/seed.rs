// Seed a managed account dir with the user's standard `~/.claude/` config.
//
// What gets copied: configuration / personalization that you'd want carried
// over to a new account dir.
//   - settings.json (env vars, permissions, hooks references, statusline,
//     plugins, language, defaults)
//   - CLAUDE.md (global memory)
//   - agents/, commands/, output-styles/, skills/ (custom user assets)
//
// What is NOT copied: per-account state and identity-bound things — those
// must stay distinct between accounts or the per-account isolation breaks.
//   - .credentials.json (auth — must be re-acquired by `claude auth login`)
//   - settings.local.json (per-machine local overrides)
//   - .account-info.json (our doctor cache)
//   - projects/, todos/, statsig/ (runtime state, sessions, telemetry)
//   - ide/ (already a symlink to ~/.claude/ide in our setup)
//   - hooks/, plugins/ (settings.json references these by absolute path,
//     so copying duplicates files that are never invoked from the copy)
//
// Existing files in the target are skipped, never overwritten — this is a
// "seed" operation, not a sync.

use std::fs;
use std::path::Path;

const COPYABLE_FILES: &[&str] = &["settings.json", "CLAUDE.md"];
const COPYABLE_DIRS: &[&str] = &["agents", "commands", "output-styles", "skills"];

pub struct CopyReport {
    pub copied: Vec<String>,
}

impl CopyReport {
    pub fn is_empty(&self) -> bool {
        self.copied.is_empty()
    }
}

pub fn copy_user_config(target: &Path) -> std::io::Result<CopyReport> {
    let source = match dirs::home_dir() {
        Some(h) => h.join(".claude"),
        None => return Err(std::io::Error::other("cannot determine home directory")),
    };
    let mut report = CopyReport { copied: vec![] };

    if !source.exists() {
        return Ok(report);
    }

    for f in COPYABLE_FILES {
        let src = source.join(f);
        let dst = target.join(f);
        if src.is_file() && !dst.exists() {
            fs::copy(&src, &dst)?;
            report.copied.push((*f).to_string());
        }
    }

    for d in COPYABLE_DIRS {
        let src = source.join(d);
        let dst = target.join(d);
        if !src.is_dir() || dst.exists() {
            continue;
        }
        // Skip empty source dirs — copying an empty `commands/` is just noise
        // in both the report and the destination.
        if fs::read_dir(&src)?.next().is_none() {
            continue;
        }
        let count = copy_dir_recursive(&src, &dst)?;
        report
            .copied
            .push(format!("{}/ ({} file{})", d, count, plural(count)));
    }

    Ok(report)
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<usize> {
    fs::create_dir_all(dst)?;
    let mut count = 0;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let dst_path = dst.join(entry.file_name());
        let file_type = entry.file_type()?;

        if file_type.is_symlink() {
            // Recreate the link instead of following it. `fs::copy` refuses a
            // symlink to a *directory* outright, which aborted the whole seed
            // partway through and left a half-copied account behind — and
            // someone who symlinked a skill into ~/.claude meant to keep one
            // copy of it, so duplicating the tree into every account would
            // fork it silently. Links point outside the config dir in
            // practice, and stay correct from anywhere.
            copy_symlink(&path, &dst_path)?;
            count += 1;
        } else if file_type.is_dir() {
            count += copy_dir_recursive(&path, &dst_path)?;
        } else if file_type.is_file() {
            fs::copy(&path, &dst_path)?;
            count += 1;
        }
        // Anything else — a socket, a fifo — is not configuration. Skipping
        // it silently beats failing the seed over something nobody meant to
        // copy.
    }
    Ok(count)
}

#[cfg(unix)]
fn copy_symlink(src: &Path, dst: &Path) -> std::io::Result<()> {
    let target = fs::read_link(src)?;
    // A leftover from an earlier partial seed would make the create fail.
    let _ = fs::remove_file(dst);
    std::os::unix::fs::symlink(target, dst)
}

#[cfg(windows)]
fn copy_symlink(src: &Path, dst: &Path) -> std::io::Result<()> {
    // Creating a symlink on Windows needs Developer Mode or elevation, so
    // follow it instead: a copy of the contents is worse than a link, but it
    // is far better than failing the seed. `metadata` follows the link.
    let meta = fs::metadata(src)?;
    if meta.is_dir() {
        copy_dir_recursive(src, dst).map(|_| ())
    } else {
        fs::copy(src, dst).map(|_| ())
    }
}

fn plural(n: usize) -> &'static str {
    if n == 1 { "" } else { "s" }
}

// Every test here creates symlinks, so the whole module is unix-only.
// Gating the tests individually and leaving the module in place makes the
// helper and the `use` dead code on Windows, which fails CI's `-D warnings`.
#[cfg(all(test, unix))]
mod tests {
    use super::*;

    fn scratch(what: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("cc-seed-{}-{}", what, std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn a_symlinked_directory_does_not_abort_the_copy_and_stays_a_link() {
        // Regression: `fs::copy` refuses a symlink to a directory, so a single
        // symlinked skill — `~/.claude/skills/foo -> ~/elsewhere/foo` is an
        // ordinary setup — failed the whole seed partway through, leaving a
        // half-populated account and exit 1.
        let dir = scratch("symlink");
        let src = dir.join("src");
        let dst = dir.join("dst");
        let outside = dir.join("outside");

        fs::create_dir_all(outside.join("nested")).unwrap();
        fs::write(outside.join("nested/file.md"), "linked").unwrap();
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("plain.md"), "plain").unwrap();
        std::os::unix::fs::symlink(&outside, src.join("linked-dir")).unwrap();
        std::os::unix::fs::symlink(outside.join("nested/file.md"), src.join("linked-file"))
            .unwrap();

        let count = copy_dir_recursive(&src, &dst).unwrap();
        assert_eq!(count, 3, "every entry should be accounted for");

        // The plain file is copied; both links are recreated as links, not
        // followed into duplicated trees.
        assert_eq!(fs::read_to_string(dst.join("plain.md")).unwrap(), "plain");
        assert!(
            fs::symlink_metadata(dst.join("linked-dir"))
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert!(
            fs::symlink_metadata(dst.join("linked-file"))
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(fs::read_link(dst.join("linked-dir")).unwrap(), outside);
        // And they still resolve from the new location.
        assert_eq!(
            fs::read_to_string(dst.join("linked-dir/nested/file.md")).unwrap(),
            "linked"
        );

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_dangling_symlink_is_carried_over_rather_than_failing_the_seed() {
        let dir = scratch("dangling");
        let src = dir.join("src");
        let dst = dir.join("dst");
        fs::create_dir_all(&src).unwrap();
        std::os::unix::fs::symlink(dir.join("gone"), src.join("broken")).unwrap();

        let count = copy_dir_recursive(&src, &dst).unwrap();
        assert_eq!(count, 1);
        assert!(
            fs::symlink_metadata(dst.join("broken"))
                .unwrap()
                .file_type()
                .is_symlink()
        );

        fs::remove_dir_all(&dir).unwrap();
    }
}
