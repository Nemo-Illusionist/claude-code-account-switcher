// Share Claude Desktop's sandbox images between profiles.
//
// A profile is a complete, isolated app data directory, and almost all of its
// weight is one file: `vm_bundles/claudevm.bundle/rootfs.img`, ten gigabytes
// of Cowork sandbox root filesystem. A second profile downloads its own copy
// of the same bytes.
//
// APFS can clone a file copy-on-write: the clone is a fully independent file
// that shares blocks with the original until one of them is written, so it
// costs nothing until it diverges. That is strictly safer than the symlink
// the Windows tools use for the same directory — those tools quit the app
// before switching, so only one instance ever touches the files, while our
// profiles run at the same time and would be writing to one shared image.
//
// What may be shared and what may not is not a judgement call: the bundle
// mixes downloaded images with per-VM identity.
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
// So the images and their provenance markers are cloned, and everything else
// is left for the app to create per profile.

use std::fs;
use std::io;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Relative to a profile directory.
pub const BUNDLE: &str = "vm_bundles/claudevm.bundle";

/// The images, and the `.origin` markers naming the image set they belong to.
/// The markers travel with them: all of them carry the same id, and images
/// whose provenance doesn't match are the app's cue to fetch again.
const SHARED: &[&str] = &[
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

pub struct SandboxReport {
    pub files: usize,
    /// Logical bytes cloned.
    pub logical: u64,
    /// What the disk actually lost, when it could be measured. `Some(0)` is
    /// the expected answer and the whole point.
    pub on_disk: Option<u64>,
}

#[derive(Debug, PartialEq)]
pub enum SandboxPlan {
    /// The source profile holds no images to share.
    NoSource,
    /// The destination already has images; replacing them would be pointless
    /// at best, and at worst would leave it half on one image set.
    Keep,
    /// Cloning would cross a filesystem boundary, so it would be a real copy
    /// of every byte — the opposite of the point.
    WouldCopy,
    Clone,
}

pub fn bundle_of(profile: &Path) -> PathBuf {
    profile.join(BUNDLE)
}

/// The shareable files present in `bundle`, with their sizes.
pub fn images(bundle: &Path) -> Vec<(PathBuf, u64)> {
    SHARED
        .iter()
        .filter_map(|name| {
            let path = bundle.join(name);
            let size = fs::metadata(&path).ok().filter(|m| m.is_file())?.len();
            Some((path, size))
        })
        .collect()
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
pub fn plan(
    source_images: usize,
    dest_images: usize,
    same_device: bool,
    force: bool,
) -> SandboxPlan {
    if source_images == 0 {
        return SandboxPlan::NoSource;
    }
    if dest_images > 0 && !force {
        return SandboxPlan::Keep;
    }
    if !same_device {
        return SandboxPlan::WouldCopy;
    }
    SandboxPlan::Clone
}

/// Clone the images into `dest_bundle`, reporting what it actually cost.
///
/// `cp -c` asks for `clonefile(2)` but **falls back to a real copy** when the
/// filesystem can't clone, silently. `plan` rules out the cross-filesystem
/// case; the free-space measurement here catches everything else, so the
/// output can state the real cost instead of assuming it was free.
pub fn clone_images(src_bundle: &Path, dest_bundle: &Path) -> io::Result<SandboxReport> {
    let files = images(src_bundle);
    if files.is_empty() {
        return Err(io::Error::other("no sandbox images to clone"));
    }
    fs::create_dir_all(dest_bundle)?;

    let before = free_bytes(dest_bundle);
    let mut logical = 0u64;
    for (path, size) in &files {
        let name = path
            .file_name()
            .ok_or_else(|| io::Error::other("image path has no file name"))?;
        clone_one(path, &dest_bundle.join(name))?;
        logical += size;
    }
    let after = free_bytes(dest_bundle);

    Ok(SandboxReport {
        files: files.len(),
        logical,
        on_disk: before.zip(after).map(|(b, a)| b.saturating_sub(a)),
    })
}

/// One file, through `cp -c`. Shelling out rather than adding `libc` for
/// `clonefile(2)`: the same trade this codebase already makes for `security`
/// and `curl`, and `cp` is always there.
fn clone_one(src: &Path, dest: &Path) -> io::Result<()> {
    let status = Command::new("/bin/cp")
        .arg("-c")
        .arg(src)
        .arg(dest)
        .status()?;
    if status.success() {
        return Ok(());
    }
    // A partial clone is worse than none — the destination would hold images
    // from no coherent image set.
    let _ = fs::remove_file(dest);
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
    // Filesystem 1024-blocks Used Avail ... — the device name can contain
    // spaces, so count from the left only after the numeric columns start.
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
        let dir = std::env::temp_dir().join(format!("cc-vm-{}-{}", tag, std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn the_bundle_sits_inside_the_profile() {
        assert_eq!(
            bundle_of(Path::new("/p")),
            Path::new("/p/vm_bundles/claudevm.bundle")
        );
    }

    #[test]
    fn only_the_shareable_files_are_picked_up() {
        // Per-VM identity and mutable state must stay behind: two live VMs
        // sharing a MAC address is a collision, not a saving.
        let dir = scratch("images");
        for name in [
            "rootfs.img",
            ".rootfs.img.origin",
            "vmlinuz",
            "macAddress",
            "machineIdentifier",
            "sessiondata.img",
            "efivars.fd",
            ".cowork-adopted",
            "vmIP",
        ] {
            fs::write(dir.join(name), "x").unwrap();
        }
        let found: Vec<String> = images(&dir)
            .into_iter()
            .map(|(p, _)| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(found, vec!["rootfs.img", "vmlinuz", ".rootfs.img.origin"]);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_empty_bundle_has_no_images() {
        let dir = scratch("empty");
        assert!(images(&dir).is_empty());
        assert!(images(&dir.join("missing")).is_empty());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn images_report_their_sizes() {
        let dir = scratch("sizes");
        fs::write(dir.join("rootfs.img"), vec![0u8; 2048]).unwrap();
        assert_eq!(images(&dir)[0].1, 2048);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn nothing_to_share_is_reported_before_anything_else() {
        assert_eq!(plan(0, 0, true, false), SandboxPlan::NoSource);
        assert_eq!(plan(0, 5, false, true), SandboxPlan::NoSource);
    }

    #[test]
    fn existing_images_are_kept_unless_forced() {
        assert_eq!(plan(5, 5, true, false), SandboxPlan::Keep);
        assert_eq!(plan(5, 5, true, true), SandboxPlan::Clone);
    }

    #[test]
    fn a_cross_filesystem_clone_is_refused_rather_than_copied() {
        // cp -c falls back to a real copy across filesystems, silently. Ten
        // gigabytes spent to save ten gigabytes is worse than doing nothing.
        assert_eq!(plan(5, 0, false, false), SandboxPlan::WouldCopy);
        assert_eq!(plan(5, 0, false, true), SandboxPlan::WouldCopy, "force too");
    }

    #[test]
    fn an_empty_destination_on_the_same_filesystem_is_cloned() {
        assert_eq!(plan(5, 0, true, false), SandboxPlan::Clone);
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
    fn cloning_copies_the_images_and_leaves_the_rest_behind() {
        let base = scratch("clone");
        let src = base.join("src");
        let dest = base.join("dest");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("rootfs.img"), vec![7u8; 4096]).unwrap();
        fs::write(src.join(".rootfs.img.origin"), "set-1").unwrap();
        fs::write(src.join("macAddress"), "aa:bb").unwrap();

        let report = clone_images(&src, &dest).unwrap();
        assert_eq!(report.files, 2);
        assert_eq!(report.logical, 4096 + 5);
        assert_eq!(fs::read(dest.join("rootfs.img")).unwrap(), vec![7u8; 4096]);
        assert_eq!(
            fs::read_to_string(dest.join(".rootfs.img.origin")).unwrap(),
            "set-1"
        );
        assert!(
            !dest.join("macAddress").exists(),
            "per-VM identity must not travel"
        );
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn cloning_from_a_bundle_without_images_is_an_error_not_an_empty_success() {
        let base = scratch("clone-empty");
        let src = base.join("src");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("macAddress"), "aa:bb").unwrap();
        assert!(clone_images(&src, &base.join("dest")).is_err());
        let _ = fs::remove_dir_all(&base);
    }
}
