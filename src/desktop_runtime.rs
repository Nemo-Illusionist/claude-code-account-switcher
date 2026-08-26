// Share Claude Desktop's downloaded runtime between profiles.
//
// A profile is a complete, isolated app data directory, and most of its
// weight is components the app downloads and then only reads:
//
//   ~10 GB   vm_bundles/claudevm.bundle/rootfs.img + kernel and initrds
//    250 MB  claude-code-vm/<version>/
//    220 MB  claude-code/<version>/
//
// Identical in every profile, and a new profile fetches its own copy of all
// of it. APFS can clone a file copy-on-write: the clone is a fully
// independent file that shares blocks with the original until one of them is
// written, so it costs nothing until it diverges.
//
// That is strictly safer than the symlink the Windows tools use for the same
// directory — those tools quit the app before switching, so only one instance
// ever touches the files, while our profiles run at the same time and would
// be writing to one shared image.
//
// What may be shared and what may not is not a judgement call. The sandbox
// bundle mixes downloaded images with per-VM identity:
//
//   rootfs.img, vmlinuz, initrd*        the images — identical per image set
//   .<name>.origin                      which image set they came from
//   ---
//   machineIdentifier, macAddress,      this VM's identity. Two live VMs
//   gvisorMacAddress, vmIP              sharing a MAC address is a network
//                                       collision, not a saving
//   sessiondata.img, efivars.fd         this profile's mutable state
//   .cowork-adopted, warm/              setup markers, cheap to recreate
//
// The versioned directories have no such mixture — they are a download and a
// `.verified` marker — so they travel whole.
//
// Not cloned at all: `Cache/` (1.2 GB) and `Code Cache/` (346 MB). Those are
// live Chromium caches, written continuously, and the app refills them itself.

use std::fs;
use std::io;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The sandbox bundle, relative to a profile directory.
pub const BUNDLE: &str = "vm_bundles/claudevm.bundle";

/// The images, and the `.origin` markers naming the image set they belong to.
/// The markers travel with them: all of them carry the same id, and images
/// whose provenance doesn't match are the app's cue to fetch again.
const BUNDLE_FILES: &[&str] = &[
    "rootfs.img",
    "vmlinuz",
    "initrd",
    "initrd-micro",
    "initrd-micro.zst",
    ".rootfs.img.origin",
    ".vmlinuz.origin",
    ".initrd.origin",
    ".initrd-micro.origin",
    ".initrd-micro.zst.origin",
];

/// Roots holding one subdirectory per downloaded version, plus the odd
/// version marker beside them (`claude-code-vm/.sdk-version`).
const VERSIONED_ROOTS: &[&str] = &["claude-code", "claude-code-vm"];

/// Finder droppings. Everything else under a versioned root belongs to the
/// download and travels.
const IGNORED: &[&str] = &[".DS_Store"];

/// Something worth cloning, addressed relative to the profile root so the
/// same value describes it in both profiles.
#[derive(Debug, PartialEq)]
pub struct Item {
    pub rel: PathBuf,
    pub bytes: u64,
    pub is_dir: bool,
}

pub struct CloneReport {
    pub items: usize,
    /// Logical bytes cloned.
    pub logical: u64,
    /// What the disk actually lost, when it could be measured. `Some(0)` is
    /// the expected answer and the whole point.
    pub on_disk: Option<u64>,
}

#[derive(Debug, PartialEq)]
pub enum RuntimePlan {
    /// The source profile has nothing to share.
    NoSource,
    /// The destination already has runtime of its own.
    Keep,
    /// Cloning would cross a filesystem boundary, so it would be a real copy
    /// of every byte — the opposite of the point.
    WouldCopy,
    Clone,
}

/// Everything shareable in `profile`, sandbox images first.
pub fn shareable(profile: &Path) -> Vec<Item> {
    let mut items: Vec<Item> = BUNDLE_FILES
        .iter()
        .filter_map(|name| {
            let rel = Path::new(BUNDLE).join(name);
            let meta = fs::metadata(profile.join(&rel))
                .ok()
                .filter(|m| m.is_file())?;
            Some(Item {
                rel,
                bytes: meta.len(),
                is_dir: false,
            })
        })
        .collect();

    for root in VERSIONED_ROOTS {
        let Ok(entries) = fs::read_dir(profile.join(root)) else {
            continue;
        };
        let mut found: Vec<Item> = entries
            .filter_map(Result::ok)
            .filter_map(|entry| {
                let name = entry.file_name();
                let name = name.to_str()?;
                if IGNORED.contains(&name) {
                    return None;
                }
                let meta = entry.metadata().ok()?;
                let rel = Path::new(root).join(name);
                let bytes = if meta.is_dir() {
                    tree_size(&entry.path())
                } else {
                    meta.len()
                };
                Some(Item {
                    rel,
                    bytes,
                    is_dir: meta.is_dir(),
                })
            })
            .collect();
        // read_dir order is arbitrary; a stable listing keeps the report and
        // the tests from depending on it.
        found.sort_by(|a, b| a.rel.cmp(&b.rel));
        items.append(&mut found);
    }
    items
}

fn tree_size(dir: &Path) -> u64 {
    let Ok(entries) = fs::read_dir(dir) else {
        return 0;
    };
    entries
        .filter_map(Result::ok)
        .map(|e| match e.metadata() {
            Ok(m) if m.is_dir() => tree_size(&e.path()),
            Ok(m) => m.len(),
            Err(_) => 0,
        })
        .sum()
}

/// Whether two paths live on the same filesystem, walking up to the nearest
/// existing ancestor since the destination usually doesn't exist yet.
/// `None` when neither can be resolved.
pub fn same_device(a: &Path, b: &Path) -> Option<bool> {
    Some(device_of(a)? == device_of(b)?)
}

fn device_of(path: &Path) -> Option<u64> {
    let mut current = Some(path);
    while let Some(p) = current {
        if let Ok(meta) = fs::metadata(p) {
            return Some(meta.dev());
        }
        current = p.parent();
    }
    None
}

/// Decided separately from the copying so every branch is testable without a
/// ten-gigabyte file.
pub fn plan(source: usize, dest: usize, same_device: bool, force: bool) -> RuntimePlan {
    if source == 0 {
        return RuntimePlan::NoSource;
    }
    if dest > 0 && !force {
        return RuntimePlan::Keep;
    }
    if !same_device {
        return RuntimePlan::WouldCopy;
    }
    RuntimePlan::Clone
}

/// Clone `items` from one profile into another, reporting what it cost.
///
/// `cp -c` asks for `clonefile(2)` but **falls back to a real copy** when the
/// filesystem can't clone, silently. `plan` rules out the cross-filesystem
/// case; the free-space measurement here catches everything else, so the
/// output can state the real cost instead of assuming it was free.
pub fn clone_into(
    src_profile: &Path,
    dest_profile: &Path,
    items: &[Item],
) -> io::Result<CloneReport> {
    if items.is_empty() {
        return Err(io::Error::other("nothing to clone"));
    }
    fs::create_dir_all(dest_profile)?;
    let before = free_bytes(dest_profile);

    let mut logical = 0u64;
    for item in items {
        let dest = dest_profile.join(&item.rel);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        // `cp -R` copies *into* an existing directory rather than over it,
        // which on a --force re-clone would nest one version inside another.
        if item.is_dir {
            let _ = fs::remove_dir_all(&dest);
        }
        clone_one(&src_profile.join(&item.rel), &dest, item.is_dir)?;
        logical += item.bytes;
    }

    Ok(CloneReport {
        items: items.len(),
        logical,
        on_disk: before
            .zip(free_bytes(dest_profile))
            .map(|(b, a)| b.saturating_sub(a)),
    })
}

/// One file or tree, through `cp -c`. Shelling out rather than adding `libc`
/// for `clonefile(2)`: the same trade this codebase already makes for
/// `security` and `curl`, and `cp` is always there.
fn clone_one(src: &Path, dest: &Path, recursive: bool) -> io::Result<()> {
    let mut cmd = Command::new("/bin/cp");
    cmd.arg("-c");
    if recursive {
        cmd.arg("-R");
    }
    let status = cmd.arg(src).arg(dest).status()?;
    if status.success() {
        return Ok(());
    }
    // A partial clone is worse than none — the destination would hold a
    // runtime from no coherent version.
    if recursive {
        let _ = fs::remove_dir_all(dest);
    } else {
        let _ = fs::remove_file(dest);
    }
    Err(io::Error::other(format!(
        "cp -c {} failed",
        src.file_name().unwrap_or_default().to_string_lossy()
    )))
}

/// Free bytes on the filesystem holding `dir`, via `df`. `None` if it can't
/// be read — the caller then reports the logical size and says nothing it
/// hasn't measured.
fn free_bytes(dir: &Path) -> Option<u64> {
    let out = Command::new("df").arg("-k").arg(dir).output().ok()?;
    if !out.status.success() {
        return None;
    }
    parse_df_available(&String::from_utf8_lossy(&out.stdout))
}

/// The `Avail` column of `df -k`, in bytes. Kept pure so the parsing is
/// tested rather than trusted.
fn parse_df_available(output: &str) -> Option<u64> {
    let line = output.lines().nth(1)?;
    // A device name can contain spaces, so the columns are counted from the
    // first numeric field rather than from the left.
    let fields: Vec<&str> = line.split_whitespace().collect();
    let first_number = fields.iter().position(|f| f.parse::<u64>().is_ok())?;
    fields
        .get(first_number + 2)?
        .parse::<u64>()
        .ok()
        .map(|kb| kb * 1024)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("cc-rt-{}-{}", tag, std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A profile with one of everything shareable, and one of everything that
    /// must not be.
    fn profile(base: &Path, name: &str) -> PathBuf {
        let p = base.join(name);
        let bundle = p.join(BUNDLE);
        fs::create_dir_all(&bundle).unwrap();
        fs::write(bundle.join("rootfs.img"), vec![0u8; 4096]).unwrap();
        fs::write(bundle.join(".rootfs.img.origin"), "set-1").unwrap();
        // Per-VM identity and mutable state.
        fs::write(bundle.join("macAddress"), "aa:bb").unwrap();
        fs::write(bundle.join("machineIdentifier"), "id").unwrap();
        fs::write(bundle.join("sessiondata.img"), vec![0u8; 512]).unwrap();

        fs::create_dir_all(p.join("claude-code/2.1.246")).unwrap();
        fs::write(p.join("claude-code/2.1.246/binary"), vec![0u8; 1024]).unwrap();
        fs::write(p.join("claude-code/.DS_Store"), "junk").unwrap();
        fs::create_dir_all(p.join("claude-code-vm/2.1.215")).unwrap();
        fs::write(p.join("claude-code-vm/2.1.215/claude"), vec![0u8; 2048]).unwrap();
        fs::write(p.join("claude-code-vm/.sdk-version"), "2.1.215").unwrap();

        // A live cache, which is never cloned.
        fs::create_dir_all(p.join("Cache")).unwrap();
        fs::write(p.join("Cache/blob"), vec![0u8; 8192]).unwrap();
        p
    }

    fn rels(items: &[Item]) -> Vec<String> {
        items
            .iter()
            .map(|i| i.rel.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn every_downloaded_component_is_offered_and_nothing_else_is() {
        let base = scratch("shareable");
        let p = profile(&base, "src");
        assert_eq!(
            rels(&shareable(&p)),
            vec![
                "vm_bundles/claudevm.bundle/rootfs.img",
                "vm_bundles/claudevm.bundle/.rootfs.img.origin",
                "claude-code/2.1.246",
                "claude-code-vm/.sdk-version",
                "claude-code-vm/2.1.215",
            ]
        );
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn a_versioned_directory_is_measured_whole() {
        let base = scratch("sizes");
        let p = profile(&base, "src");
        let items = shareable(&p);
        let dir = items.iter().find(|i| i.rel.ends_with("2.1.246")).unwrap();
        assert!(dir.is_dir);
        assert_eq!(dir.bytes, 1024, "the tree, not the directory entry");
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn a_profile_with_nothing_downloaded_offers_nothing() {
        let base = scratch("bare");
        fs::create_dir_all(base.join("empty")).unwrap();
        assert!(shareable(&base.join("empty")).is_empty());
        assert!(shareable(&base.join("missing")).is_empty());
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn nothing_to_share_is_reported_before_anything_else() {
        assert_eq!(plan(0, 0, true, false), RuntimePlan::NoSource);
        assert_eq!(plan(0, 5, false, true), RuntimePlan::NoSource);
    }

    #[test]
    fn an_existing_runtime_is_kept_unless_forced() {
        assert_eq!(plan(5, 5, true, false), RuntimePlan::Keep);
        assert_eq!(plan(5, 5, true, true), RuntimePlan::Clone);
    }

    #[test]
    fn a_cross_filesystem_clone_is_refused_rather_than_copied() {
        // cp -c falls back to a real copy across filesystems, silently. Ten
        // gigabytes spent to save ten gigabytes is worse than doing nothing.
        assert_eq!(plan(5, 0, false, false), RuntimePlan::WouldCopy);
        assert_eq!(plan(5, 0, false, true), RuntimePlan::WouldCopy, "force too");
    }

    #[test]
    fn an_empty_destination_on_the_same_filesystem_is_cloned() {
        assert_eq!(plan(5, 0, true, false), RuntimePlan::Clone);
    }

    #[test]
    fn a_path_and_its_own_parent_are_on_one_filesystem() {
        let dir = scratch("dev");
        // The destination normally doesn't exist yet, which is the case that
        // has to resolve through the nearest existing ancestor.
        assert_eq!(same_device(&dir, &dir.join("not/created/yet")), Some(true));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_unresolvable_path_has_no_device() {
        assert_eq!(same_device(Path::new("/"), Path::new("")), None);
    }

    #[test]
    fn df_available_is_read_from_the_right_column() {
        let out = "Filesystem 1024-blocks      Used     Avail Capacity  Mounted on\n\
                   /dev/disk3s5  970172928 401545212 361738396    53%    /System/Volumes/Data\n";
        assert_eq!(parse_df_available(out), Some(361738396 * 1024));
    }

    #[test]
    fn df_output_it_cannot_read_is_not_guessed_at() {
        assert_eq!(parse_df_available(""), None);
        assert_eq!(parse_df_available("Filesystem Avail\n"), None);
        assert_eq!(parse_df_available("only a header line\n"), None);
    }

    #[test]
    fn cloning_brings_the_runtime_and_leaves_identity_and_caches_behind() {
        let base = scratch("clone");
        let src = profile(&base, "src");
        let dest = base.join("dest");

        let items = shareable(&src);
        let report = clone_into(&src, &dest, &items).unwrap();
        assert_eq!(report.items, 5);
        assert_eq!(report.logical, 4096 + 5 + 1024 + 7 + 2048);

        assert_eq!(
            fs::read(dest.join(BUNDLE).join("rootfs.img"))
                .unwrap()
                .len(),
            4096
        );
        assert!(dest.join("claude-code/2.1.246/binary").is_file());
        assert!(dest.join("claude-code-vm/2.1.215/claude").is_file());
        assert!(dest.join("claude-code-vm/.sdk-version").is_file());

        for left_behind in [
            "vm_bundles/claudevm.bundle/macAddress",
            "vm_bundles/claudevm.bundle/machineIdentifier",
            "vm_bundles/claudevm.bundle/sessiondata.img",
            "claude-code/.DS_Store",
            "Cache/blob",
        ] {
            assert!(
                !dest.join(left_behind).exists(),
                "{} must not travel",
                left_behind
            );
        }
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn re_cloning_replaces_a_version_rather_than_nesting_it() {
        // Regression shape: `cp -R src dest` with dest present copies *into*
        // it, producing claude-code/2.1.246/2.1.246.
        let base = scratch("reclone");
        let src = profile(&base, "src");
        let dest = base.join("dest");
        let items = shareable(&src);
        clone_into(&src, &dest, &items).unwrap();
        clone_into(&src, &dest, &items).unwrap();
        assert!(dest.join("claude-code/2.1.246/binary").is_file());
        assert!(!dest.join("claude-code/2.1.246/2.1.246").exists());
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn cloning_nothing_is_an_error_not_an_empty_success() {
        let base = scratch("clone-empty");
        assert!(clone_into(&base, &base.join("dest"), &[]).is_err());
        let _ = fs::remove_dir_all(&base);
    }
}
