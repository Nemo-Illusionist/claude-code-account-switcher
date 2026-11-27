// Seed a managed account dir from another config dir — the user's standard
// `~/.claude/` by default, or a named account with `clone-settings --from`.
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
//   - hooks/ (settings.json references these by absolute path, so copying
//     duplicates files that are never invoked from the copy)
//
// `plugins/` is the exception to that last rule, and gets its own treatment
// below: Claude Code keeps a plugin registry per config dir, so a fresh
// account starts with none, and the registry records absolute paths into
// the config dir it was written for. A plain copy would leave the new
// account loading the old one's plugin cache — working by accident, and
// breaking the moment that account is removed.
//
// Existing files in the target are skipped, never overwritten — this is a
// "seed" operation, not a sync.

use std::fs;
use std::path::Path;

const COPYABLE_FILES: &[&str] = &["settings.json", "CLAUDE.md"];
const COPYABLE_DIRS: &[&str] = &["agents", "commands", "output-styles", "skills"];

/// The plugin registry files, and the JSON keys in them that hold a path
/// into the config dir. Both are rewritten when plugins are seeded.
const PLUGIN_REGISTRIES: &[&str] = &["installed_plugins.json", "known_marketplaces.json"];
const PLUGIN_PATH_KEYS: &[&str] = &["installPath", "installLocation"];

pub struct CopyReport {
    pub copied: Vec<String>,
}

impl CopyReport {
    pub fn is_empty(&self) -> bool {
        self.copied.is_empty()
    }
}

/// Seed `target` from the user's standard `~/.claude/`.
pub fn copy_user_config(target: &Path) -> std::io::Result<CopyReport> {
    let source = match dirs::home_dir() {
        Some(h) => h.join(".claude"),
        None => return Err(std::io::Error::other("cannot determine home directory")),
    };
    copy_config(&source, target)
}

/// Seed `target` from `source`.
///
/// The source used to be `~/.claude/` and nothing else, which made the
/// obvious thing — "give this new account the same setup as the account I
/// already use" — inexpressible: you had to copy the files by hand, and get
/// the plugin registry's rewritten paths right by hand with them.
pub fn copy_config(source: &Path, target: &Path) -> std::io::Result<CopyReport> {
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

    if let Some(count) = copy_plugins(source, target)? {
        report
            .copied
            .push(format!("plugins/ ({} file{})", count, plural(count)));
    }

    Ok(report)
}

/// Seed `target/plugins/` from `source/plugins/`, rewriting the registry so
/// every path points into the new account rather than the old one.
///
/// Returns the number of files copied, or `None` when there was nothing to
/// do — no source plugins, or the account already has plugins of its own.
/// An account that has installed something is not ours to merge into; a
/// freshly created one has an empty registry and can be seeded safely.
pub fn copy_plugins(source: &Path, target: &Path) -> std::io::Result<Option<usize>> {
    let src = source.join("plugins");
    let dst = target.join("plugins");
    if !src.is_dir() || !has_plugins(&src) || has_plugins(&dst) {
        return Ok(None);
    }

    let count = copy_dir_recursive(&src, &dst)?;

    // The copy carries the source's absolute paths. Rewrite them, or the new
    // account loads the old account's plugin cache.
    let old_prefix = src.to_string_lossy().to_string();
    let new_prefix = dst.to_string_lossy().to_string();
    for name in PLUGIN_REGISTRIES {
        let file = dst.join(name);
        let Ok(raw) = fs::read_to_string(&file) else {
            continue;
        };
        if let Some(rewritten) = rewrite_plugin_paths(&raw, &old_prefix, &new_prefix) {
            fs::write(&file, rewritten)?;
        }
    }
    Ok(Some(count))
}

/// Whether a `plugins/` directory has anything installed. A directory Claude
/// Code created but never filled has an empty `plugins` map, and counts as
/// nothing installed.
fn has_plugins(plugins_dir: &Path) -> bool {
    let Ok(raw) = fs::read_to_string(plugins_dir.join("installed_plugins.json")) else {
        return false;
    };
    registry_lists_a_plugin(&raw)
}

/// Value-in, value-out so the "is this registry empty" decision is testable
/// without a filesystem.
pub fn registry_lists_a_plugin(raw: &str) -> bool {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(raw) else {
        // Unparseable: treat it as "has plugins" so we leave it alone rather
        // than copying over something we could not read.
        return true;
    };
    v.get("plugins")
        .and_then(|p| p.as_object())
        .is_some_and(|m| !m.is_empty())
}

/// Repoint every path-valued key that lives under `old_prefix` at
/// `new_prefix`, leaving the rest of the document untouched.
///
/// Done structurally rather than by text substitution: a marketplace can be
/// sourced from a directory outside the config dir — a plugin marketplace
/// checked into a project, say — and those paths must survive verbatim.
/// Returns `None` when the text isn't JSON we can rewrite.
pub fn rewrite_plugin_paths(raw: &str, old_prefix: &str, new_prefix: &str) -> Option<String> {
    let mut v: serde_json::Value = serde_json::from_str(raw).ok()?;
    rewrite_in_place(&mut v, old_prefix, new_prefix);
    serde_json::to_string_pretty(&v).ok().map(|mut s| {
        s.push('\n');
        s
    })
}

fn rewrite_in_place(v: &mut serde_json::Value, old_prefix: &str, new_prefix: &str) {
    match v {
        serde_json::Value::Object(map) => {
            for (key, value) in map.iter_mut() {
                if PLUGIN_PATH_KEYS.contains(&key.as_str())
                    && let Some(path) = value.as_str()
                    && let Some(rest) = path.strip_prefix(old_prefix)
                {
                    *value = serde_json::Value::String(format!("{}{}", new_prefix, rest));
                    continue;
                }
                rewrite_in_place(value, old_prefix, new_prefix);
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                rewrite_in_place(item, old_prefix, new_prefix);
            }
        }
        _ => {}
    }
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
            // fork it silently.
            //
            // A *relative* link survives only a copy of the same depth.
            // `~/.claude/skills/foo -> ../../elsewhere/foo` is two levels up
            // from the skills dir; inside `accounts/<name>/skills/` the same
            // text is two levels short, and resolves nowhere. So the
            // `--from default` direction can carry a broken link into an
            // account, while account-to-account is equal in depth and
            // cannot. (The reverse exists too: a link written inside an
            // account — four `../` here, from a real `skills/computer-use`
            // — would be wrong if it were ever copied up to `~/.claude`.)
            // Rewriting it would be a guess about what the author meant, so
            // the link is copied as written; this is here so the next reader
            // knows it is a known edge and not an oversight.
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

// Not gated as a whole, unlike most of this codebase: the plugin tests below
// run everywhere and share `scratch` with the symlink ones, so nothing is
// left stranded on Windows. Only the two symlink tests are unix-only.
// See .claude/rules/platform-gating.md.
#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(what: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("cc-seed-{}-{}", what, std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn an_account_can_be_seeded_from_another_account() {
        // What `clone-settings --from` is for: give a new account the setup
        // of the one already in use, with the plugin registry repointed at
        // the new dir rather than loading the other account's cache.
        let dir = scratch("from-account");
        let src = dir.join("accounts/work");
        let dst = dir.join("accounts/work-2");
        fs::create_dir_all(&dst).unwrap();
        fs::create_dir_all(src.join("agents")).unwrap();
        fs::write(src.join("settings.json"), r#"{"theme":"dark"}"#).unwrap();
        fs::write(src.join("CLAUDE.md"), "memory").unwrap();
        fs::write(src.join("agents/one.md"), "agent").unwrap();
        // A registry whose paths point into the source account, and a
        // marketplace sourced from outside any config dir.
        fs::create_dir_all(src.join("plugins/cache/local/helper/1.0.0")).unwrap();
        fs::write(src.join("plugins/cache/local/helper/1.0.0/plugin.md"), "p").unwrap();
        // Two platform details, both of which the Windows runner caught and
        // nothing here can:
        //
        // `serde_json`, not `format!`: a Windows path holds backslashes, and
        // pasting one into a JSON string literal is an invalid escape, so
        // the fixture was not JSON at all there.
        //
        // And `Path::join`, not a `/` glued onto a path: the rewrite takes
        // its prefix from `source.join("plugins").to_string_lossy()` and
        // compares strings, so a fixture mixing `\` with `/` would not match
        // the prefix on Windows — the file would be left alone and the
        // assertions below would fail against an unrewritten path. Same
        // pattern as `an_install_path_inside_the_old_config_dir_is_repointed_at_the_new_one`.
        let p = |root: &Path, rest: &[&str]| -> String {
            let mut out = root.to_path_buf();
            for part in rest {
                out = out.join(part);
            }
            out.to_string_lossy().into_owned()
        };
        fs::write(
            src.join("plugins/installed_plugins.json"),
            serde_json::json!({
                "plugins": {
                    "helper@local": [{
                        "scope": "user",
                        "installPath": p(&src, &["plugins", "cache", "local", "helper", "1.0.0"]),
                    }]
                }
            })
            .to_string(),
        )
        .unwrap();
        fs::write(
            src.join("plugins/known_marketplaces.json"),
            serde_json::json!({
                "local": {
                    "installLocation": p(&src, &["plugins", "marketplaces", "local"])
                },
                "repo": { "installLocation": "/work/repo/.claude/plugins" }
            })
            .to_string(),
        )
        .unwrap();

        let report = copy_config(&src, &dst).unwrap();
        assert!(!report.is_empty(), "nothing was copied");

        assert_eq!(fs::read_to_string(dst.join("CLAUDE.md")).unwrap(), "memory");
        assert_eq!(
            fs::read_to_string(dst.join("settings.json")).unwrap(),
            r#"{"theme":"dark"}"#
        );
        assert!(dst.join("agents/one.md").is_file());

        // The paths that pointed into the source now point into the target,
        // and the one outside any config dir is untouched. Expected values
        // written out here, not derived from the rewriter.
        let installed: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(dst.join("plugins/installed_plugins.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(
            installed["plugins"]["helper@local"][0]["installPath"]
                .as_str()
                .unwrap(),
            p(&dst, &["plugins", "cache", "local", "helper", "1.0.0"])
        );
        assert_eq!(installed["plugins"]["helper@local"][0]["scope"], "user");
        let markets: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(dst.join("plugins/known_marketplaces.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(
            markets["local"]["installLocation"].as_str().unwrap(),
            p(&dst, &["plugins", "marketplaces", "local"])
        );
        assert_eq!(
            markets["repo"]["installLocation"],
            "/work/repo/.claude/plugins"
        );
        assert!(
            dst.join("plugins/cache/local/helper/1.0.0/plugin.md")
                .is_file()
        );

        // A second run has nothing left to do: this is a seed, not a sync.
        let again = copy_config(&src, &dst).unwrap();
        assert!(again.is_empty(), "copied twice: {:?}", again.copied);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    #[cfg(unix)]
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
    #[cfg(unix)]
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

    const REGISTRY: &str = r#"{
      "version": 2,
      "plugins": {
        "helper@official": [
          { "scope": "user", "installPath": "/old/.claude/plugins/cache/official/helper/abc" }
        ]
      }
    }"#;

    #[test]
    fn an_install_path_inside_the_old_config_dir_is_repointed_at_the_new_one() {
        let out =
            rewrite_plugin_paths(REGISTRY, "/old/.claude/plugins", "/new/acct/plugins").unwrap();
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(
            v["plugins"]["helper@official"][0]["installPath"],
            "/new/acct/plugins/cache/official/helper/abc"
        );
        // Everything else survives.
        assert_eq!(v["version"], 2);
        assert_eq!(v["plugins"]["helper@official"][0]["scope"], "user");
    }

    #[test]
    fn a_marketplace_sourced_from_outside_the_config_dir_keeps_its_path() {
        // A marketplace checked into a project lives outside the config dir.
        // Rewriting it would point the account at a directory that has
        // nothing to do with it — this is why the rewrite is structural and
        // prefix-matched rather than a text substitution.
        let raw = r#"{
          "official": { "installLocation": "/old/.claude/plugins/marketplaces/official" },
          "local": {
            "source": { "source": "directory", "path": "/work/repo/.claude/plugins" },
            "installLocation": "/work/repo/.claude/plugins"
          }
        }"#;
        let out = rewrite_plugin_paths(raw, "/old/.claude/plugins", "/new/acct/plugins").unwrap();
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(
            v["official"]["installLocation"],
            "/new/acct/plugins/marketplaces/official"
        );
        assert_eq!(v["local"]["installLocation"], "/work/repo/.claude/plugins");
        assert_eq!(v["local"]["source"]["path"], "/work/repo/.claude/plugins");
    }

    #[test]
    fn a_path_that_merely_starts_with_a_similar_name_is_not_rewritten() {
        let raw = r#"{ "a": { "installLocation": "/old/.claude/plugins-backup/x" } }"#;
        let out = rewrite_plugin_paths(raw, "/old/.claude/plugins/", "/new/acct/plugins/").unwrap();
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["a"]["installLocation"], "/old/.claude/plugins-backup/x");
    }

    #[test]
    fn text_that_is_not_json_is_refused_rather_than_mangled() {
        assert!(rewrite_plugin_paths("not json", "/a", "/b").is_none());
    }

    #[test]
    fn an_empty_registry_counts_as_nothing_installed() {
        assert!(!registry_lists_a_plugin(r#"{"version": 2, "plugins": {}}"#));
        assert!(!registry_lists_a_plugin(r#"{"version": 2}"#));
        assert!(registry_lists_a_plugin(REGISTRY));
    }

    #[test]
    fn an_unreadable_registry_counts_as_installed_so_it_is_left_alone() {
        // Better to seed nothing than to copy over a registry we could not
        // parse and cannot reason about.
        assert!(registry_lists_a_plugin("{ broken"));
    }

    #[test]
    fn seeding_copies_the_tree_and_repoints_the_registry() {
        let dir = scratch("plugins");
        let source = dir.join("source");
        let target = dir.join("target");
        let src_plugins = source.join("plugins");
        fs::create_dir_all(src_plugins.join("cache/official/helper/abc")).unwrap();
        fs::write(
            src_plugins.join("cache/official/helper/abc/plugin.md"),
            "hi",
        )
        .unwrap();
        // Build the registry with serde rather than by substituting the path
        // into a JSON string literal: a Windows path pastes `C:\Users\...`
        // straight into the document, and `\U` is not a valid JSON escape.
        let registry = serde_json::json!({
            "version": 2,
            "plugins": {
                "helper@official": [{
                    "scope": "user",
                    "installPath": src_plugins.join("cache/official/helper/abc"),
                }]
            }
        });
        fs::write(
            src_plugins.join("installed_plugins.json"),
            serde_json::to_string_pretty(&registry).unwrap(),
        )
        .unwrap();
        fs::create_dir_all(&target).unwrap();

        let count = copy_plugins(&source, &target).unwrap().unwrap();
        assert!(count >= 2, "copied {count} files");

        let written = fs::read_to_string(target.join("plugins/installed_plugins.json")).unwrap();
        let v: serde_json::Value = serde_json::from_str(&written).unwrap();
        let path = v["plugins"]["helper@official"][0]["installPath"]
            .as_str()
            .unwrap();
        assert!(
            path.starts_with(&*target.to_string_lossy()),
            "registry still points at the source: {path}"
        );
        assert!(std::path::Path::new(path).join("plugin.md").is_file());

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn an_account_with_its_own_plugins_is_left_alone() {
        // Seeding is not a merge. Someone who has installed plugins under
        // this account keeps exactly what they installed.
        let dir = scratch("plugins-existing");
        let source = dir.join("source");
        let target = dir.join("target");
        fs::create_dir_all(source.join("plugins")).unwrap();
        fs::write(source.join("plugins/installed_plugins.json"), REGISTRY).unwrap();
        fs::create_dir_all(target.join("plugins")).unwrap();
        let theirs = r#"{"version": 2, "plugins": {"mine@local": [{"scope": "user"}]}}"#;
        fs::write(target.join("plugins/installed_plugins.json"), theirs).unwrap();

        assert_eq!(copy_plugins(&source, &target).unwrap(), None);
        assert_eq!(
            fs::read_to_string(target.join("plugins/installed_plugins.json")).unwrap(),
            theirs
        );

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_source_with_no_plugins_installed_seeds_nothing() {
        let dir = scratch("plugins-empty-source");
        let source = dir.join("source");
        let target = dir.join("target");
        fs::create_dir_all(source.join("plugins")).unwrap();
        fs::write(
            source.join("plugins/installed_plugins.json"),
            r#"{"version": 2, "plugins": {}}"#,
        )
        .unwrap();
        fs::create_dir_all(&target).unwrap();

        assert_eq!(copy_plugins(&source, &target).unwrap(), None);
        assert!(!target.join("plugins").exists());

        fs::remove_dir_all(&dir).unwrap();
    }
}
