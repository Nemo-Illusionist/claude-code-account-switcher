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
        if entry.file_type()?.is_dir() {
            count += copy_dir_recursive(&path, &dst_path)?;
        } else {
            fs::copy(&path, &dst_path)?;
            count += 1;
        }
    }
    Ok(count)
}

fn plural(n: usize) -> &'static str {
    if n == 1 { "" } else { "s" }
}
