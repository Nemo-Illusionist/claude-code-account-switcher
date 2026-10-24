//! The one owner of the account snapshot. Builds run one at a time, each
//! committed snapshot gets the next revision, and every commit reaches the
//! tray and the webview the same way — so neither depends on the other being
//! open, and a stale build can never overwrite a newer one.
use crate::launcher::{self, Installation};
use claude_acc::config::AppConfig;
use claude_acc::gui::{self, GuiError, Snapshot};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};
use std::time::Instant;
use tauri::{AppHandle, Emitter, Manager};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AppError {
    CliMissing,
    // Constructed only by the macOS launcher; elsewhere `open_terminal`
    // fails earlier with `TerminalMacosOnly`.
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    LauncherFailed,
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    TerminalFailed,
    // Constructed only where there is no Terminal launcher.
    #[cfg_attr(target_os = "macos", allow(dead_code))]
    TerminalMacosOnly,
    BackendFailed,
    // Untagged variants must come last; serializes as the bare GuiError code.
    #[serde(untagged)]
    Gui(GuiError),
}

impl From<GuiError> for AppError {
    fn from(e: GuiError) -> Self {
        Self::Gui(e)
    }
}

/// A library snapshot plus this shell's ordering number. Flattened, so the
/// webview sees the library's fields with `revision` beside them.
#[derive(Debug, Clone, Serialize)]
pub struct Revisioned {
    pub revision: u64,
    #[serde(flatten)]
    pub snapshot: Snapshot,
}

struct Latest {
    snapshot: Revisioned,
    /// When the build behind `snapshot` finished. A default change re-stamps
    /// the snapshot but keeps this, so it never passes for a fresh reading.
    built: Instant,
    live: bool,
}

/// A value read once and kept until it is invalidated.
pub struct Cached<T>(Mutex<Option<T>>);

impl<T: Clone> Cached<T> {
    pub fn new() -> Self {
        Self(Mutex::new(None))
    }

    fn lock(&self) -> MutexGuard<'_, Option<T>> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn get_or(&self, read: impl FnOnce() -> T) -> T {
        self.lock().get_or_insert_with(read).clone()
    }

    pub fn invalidate(&self) {
        *self.lock() = None;
    }
}

pub struct Backend {
    pub config: AppConfig,
    /// The standard account's config dir, `~/.claude`.
    pub standard: PathBuf,
    /// Held for a whole build, network included. Nothing else takes it:
    /// bindings, sessions and writes never wait on a slow refresh.
    build: Mutex<()>,
    latest: Mutex<Option<Latest>>,
    /// Reading it runs the installed CLI, so it is kept between Details
    /// views, and dropped on a live refresh: after `claude-acc update` the
    /// next refresh reports the new version without restarting the app.
    installation: Cached<Installation>,
}

impl Backend {
    pub fn new(config: AppConfig, standard: PathBuf) -> Self {
        Self {
            config,
            standard,
            build: Mutex::new(()),
            latest: Mutex::new(None),
            installation: Cached::new(),
        }
    }

    /// The `HOME` this app resolved accounts under. Passed on to Terminal so
    /// the CLI there resolves the same ones.
    pub fn home(&self) -> Option<&std::path::Path> {
        self.standard.parent()
    }

    pub fn installation(&self) -> Installation {
        self.installation
            .get_or(|| launcher::installation(&self.config.base_dir))
    }

    fn latest(&self) -> MutexGuard<'_, Option<Latest>> {
        self.latest.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Store a fresh build as the next revision.
    fn commit_build(&self, snapshot: Snapshot, live: bool) -> Revisioned {
        let mut latest = self.latest();
        let previous = latest.as_ref().map_or(0, |l| l.snapshot.revision);
        let next = Latest {
            snapshot: Revisioned {
                revision: previous,
                snapshot,
            },
            built: Instant::now(),
            live,
        };
        self.stamp(&mut latest, next)
    }

    /// Re-stamp the last snapshot as the next revision after a change made
    /// outside a build. One lock throughout, so a build committing meanwhile
    /// is never overwritten by an older copy.
    fn commit_change(&self) -> Option<Revisioned> {
        let mut latest = self.latest();
        let last = latest.take()?;
        Some(self.stamp(&mut latest, last))
    }

    /// The default, the account list and every pin state are read from disk
    /// at every commit: a build that started before a default change, a
    /// removal or a pin must not put the old value back.
    fn stamp(&self, latest: &mut Option<Latest>, mut next: Latest) -> Revisioned {
        if let Ok(default) = self.config.get_default() {
            next.snapshot
                .snapshot
                .mark_default(default.as_deref().unwrap_or("default"));
        }
        next.snapshot
            .snapshot
            .accounts
            .retain(|a| a.name == "default" || self.config.account_path(&a.name).is_dir());
        for a in &mut next.snapshot.snapshot.accounts {
            a.lock = gui::lock_of(&self.config, &self.standard, &a.name);
        }
        next.snapshot.revision += 1;
        let snapshot = next.snapshot.clone();
        *latest = Some(next);
        snapshot
    }
}

/// Build (or reuse) a snapshot, commit it, and publish it everywhere.
pub fn refresh(app: &AppHandle, live: bool) -> Result<Revisioned, AppError> {
    let backend = app.state::<Backend>();
    if live {
        backend.installation.invalidate();
    }
    let requested = Instant::now();
    let _build = backend.build.lock().unwrap_or_else(|e| e.into_inner());
    // Queued behind another build: if that one finished after we asked and
    // is at least as live as what we want, it already answers this request.
    if let Some(l) = backend.latest().as_ref()
        && l.built > requested
        && (l.live || !live)
    {
        return Ok(l.snapshot.clone());
    }
    let snapshot = gui::snapshot(&backend.config, &backend.standard, live)?;
    let snapshot = backend.commit_build(snapshot, live);
    publish(app, &snapshot);
    Ok(snapshot)
}

pub fn set_default(app: &AppHandle, account: &str) -> Result<Revisioned, AppError> {
    let backend = app.state::<Backend>();
    gui::change_default(&backend.config, &backend.standard, account)?;
    match backend.commit_change() {
        Some(snapshot) => {
            publish(app, &snapshot);
            Ok(snapshot)
        }
        None => refresh(app, false),
    }
}

pub fn remove_account(app: &AppHandle, account: &str) -> Result<Revisioned, AppError> {
    let backend = app.state::<Backend>();
    gui::remove_account(&backend.config, &backend.standard, account)?;
    match backend.commit_change() {
        Some(snapshot) => {
            publish(app, &snapshot);
            Ok(snapshot)
        }
        None => refresh(app, false),
    }
}

pub fn pin_account(app: &AppHandle, account: &str, force: bool) -> Result<Revisioned, AppError> {
    let backend = app.state::<Backend>();
    gui::pin_account(&backend.config, &backend.standard, account, force)?;
    match backend.commit_change() {
        Some(snapshot) => {
            publish(app, &snapshot);
            Ok(snapshot)
        }
        None => refresh(app, false),
    }
}

fn publish(app: &AppHandle, snapshot: &Revisioned) {
    crate::tray::update(app, &snapshot.snapshot);
    let _ = app.emit("snapshot", snapshot);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn a_cached_value_is_read_once_until_invalidated() {
        // Regression: the installed CLI's version sat in a process-lifetime
        // OnceLock, so after `claude-acc update` the app kept the old one.
        let reads = Cell::new(0);
        let cache = Cached::new();
        let read = |v: &'static str| {
            reads.set(reads.get() + 1);
            v
        };
        assert_eq!(cache.get_or(|| read("0.21.0")), "0.21.0");
        assert_eq!(cache.get_or(|| read("unused")), "0.21.0");
        assert_eq!(reads.get(), 1, "a second view must not spawn the CLI again");
        cache.invalidate();
        assert_eq!(cache.get_or(|| read("0.22.0")), "0.22.0");
        assert_eq!(reads.get(), 2);
    }

    #[test]
    fn a_build_that_saw_a_removed_account_does_not_bring_it_back() {
        let root = std::env::temp_dir().join(format!("cc-mgr-stamp-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let config = AppConfig {
            base_dir: root.join("switch"),
        };
        config.init().unwrap();
        std::fs::create_dir_all(config.account_path("work")).unwrap();
        let backend = Backend::new(config, root.join("standard"));
        let row = |name: &str| {
            let mut a = gui::snapshot(&backend.config, &backend.standard, false)
                .unwrap()
                .accounts
                .remove(0);
            a.name = name.into();
            a
        };
        // Built while "gone" still existed; committed after it was removed.
        let built = Snapshot {
            observed_at: 1,
            default_account: "default".into(),
            accounts: vec![row("default"), row("work"), row("gone")],
        };
        let names: Vec<_> = backend
            .commit_build(built, false)
            .snapshot
            .accounts
            .into_iter()
            .map(|a| a.name)
            .collect();
        assert_eq!(names, ["default", "work"]);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_build_that_read_the_old_pin_does_not_put_it_back() {
        let root = std::env::temp_dir().join(format!("cc-mgr-pin-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let config = AppConfig {
            base_dir: root.join("switch"),
        };
        config.init().unwrap();
        let work = config.account_path("work");
        std::fs::create_dir_all(&work).unwrap();
        std::fs::write(
            work.join(".claude.json"),
            r#"{"oauthAccount":{"accountUuid":"u-1"}}"#,
        )
        .unwrap();
        let backend = Backend::new(config, root.join("standard"));
        // Built before the pin: unpinned.
        let built = gui::snapshot(&backend.config, &backend.standard, false).unwrap();
        assert_eq!(built.accounts[1].lock, gui::LockStatus::Unpinned);
        gui::pin_account(&backend.config, &backend.standard, "work", false).unwrap();
        let committed = backend.commit_build(built, false);
        assert_eq!(
            committed.snapshot.accounts[1].lock,
            gui::LockStatus::Matched
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn the_revision_sits_beside_the_library_fields() {
        let wrapped = Revisioned {
            revision: 7,
            snapshot: Snapshot {
                observed_at: 1,
                default_account: "default".into(),
                accounts: vec![],
            },
        };
        assert_eq!(
            serde_json::to_value(wrapped).unwrap(),
            serde_json::json!({
                "revision": 7,
                "observed_at": 1,
                "default_account": "default",
                "accounts": []
            })
        );
    }
}
