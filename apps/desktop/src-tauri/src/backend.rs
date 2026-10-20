//! The one owner of the account snapshot. Builds run one at a time, each
//! committed snapshot gets the next revision, and every commit reaches the
//! tray and the webview the same way — so neither depends on the other being
//! open, and a stale build can never overwrite a newer one.
use claude_acc::config::AppConfig;
use claude_acc::gui::{self, GuiError, Snapshot};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Instant;
use tauri::{AppHandle, Emitter, Manager};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AppError {
    CliMissing,
    LauncherFailed,
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

struct Latest {
    snapshot: Snapshot,
    /// When the build behind `snapshot` finished. A default change re-stamps
    /// the snapshot but keeps this, so it never passes for a fresh reading.
    built: Instant,
    live: bool,
}

pub struct Backend {
    pub config: AppConfig,
    /// The standard account's config dir, `~/.claude`.
    pub standard: PathBuf,
    /// Held for a whole build, network included. Nothing else takes it:
    /// bindings, sessions and writes never wait on a slow refresh.
    build: Mutex<()>,
    latest: Mutex<Option<Latest>>,
}

impl Backend {
    pub fn new(config: AppConfig, standard: PathBuf) -> Self {
        Self {
            config,
            standard,
            build: Mutex::new(()),
            latest: Mutex::new(None),
        }
    }

    /// The `HOME` this app resolved accounts under. Passed on to Terminal so
    /// the CLI there resolves the same ones.
    pub fn home(&self) -> Option<&std::path::Path> {
        self.standard.parent()
    }

    fn latest(&self) -> std::sync::MutexGuard<'_, Option<Latest>> {
        self.latest.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Store a fresh build as the next revision.
    fn commit_build(&self, snapshot: Snapshot, live: bool) -> Snapshot {
        let mut latest = self.latest();
        let previous = latest.as_ref().map_or(0, |l| l.snapshot.revision);
        let built = Instant::now();
        self.stamp(
            &mut latest,
            Latest {
                snapshot,
                built,
                live,
            },
            previous,
        )
    }

    /// Re-stamp the last snapshot as the next revision after a change made
    /// outside a build. One lock throughout, so a build committing meanwhile
    /// is never overwritten by an older copy.
    fn commit_change(&self) -> Option<Snapshot> {
        let mut latest = self.latest();
        let last = latest.take()?;
        let previous = last.snapshot.revision;
        Some(self.stamp(&mut latest, last, previous))
    }

    /// The default is read from disk at every commit: a build that started
    /// before a default change must not put the old default back.
    fn stamp(&self, latest: &mut Option<Latest>, mut next: Latest, previous: u64) -> Snapshot {
        if let Ok(default) = self.config.get_default() {
            next.snapshot
                .mark_default(default.as_deref().unwrap_or("default"));
        }
        next.snapshot.revision = previous + 1;
        let snapshot = next.snapshot.clone();
        *latest = Some(next);
        snapshot
    }
}

/// Build (or reuse) a snapshot, commit it, and publish it everywhere.
pub fn refresh(app: &AppHandle, live: bool) -> Result<Snapshot, AppError> {
    let backend = app.state::<Backend>();
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

pub fn set_default(app: &AppHandle, account: &str) -> Result<Snapshot, AppError> {
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

fn publish(app: &AppHandle, snapshot: &Snapshot) {
    crate::tray::update(app, snapshot);
    let _ = app.emit("snapshot", snapshot);
}
