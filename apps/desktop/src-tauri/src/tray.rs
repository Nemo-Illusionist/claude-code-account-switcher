//! The menu bar: account entries that open Claude, plus refresh and
//! navigation. Refresh and launch run here, on worker threads, so the menu
//! works with the window closed and never blocks the main thread.
use crate::backend::{self, Backend};
use crate::launcher;
use claude_acc::gui::{self, Account, LaunchAction, Snapshot, UsageSource};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager};

const TRAY: &str = "accounts";

pub fn install(app: &AppHandle) -> tauri::Result<()> {
    TrayIconBuilder::with_id(TRAY)
        .icon(app.default_window_icon().expect("app icon").clone())
        .tooltip("Claude Accounts — per-project accounts")
        .on_menu_event(|app, event| on_click(app, event.id().as_ref()))
        .build(app)?;
    Ok(())
}

pub fn show(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.set_focus();
    }
}

fn on_click(app: &AppHandle, id: &str) {
    match id {
        "quit" => app.exit(0),
        "show" => show(app),
        "bindings" => {
            show(app);
            let _ = app.emit("navigate", "projects");
        }
        "refresh" => {
            let app = app.clone();
            std::thread::spawn(move || {
                if let Err(e) = backend::refresh(&app, true) {
                    let _ = app.emit("backend-error", e);
                }
            });
        }
        _ => {
            if let Some(name) = id.strip_prefix("launch:") {
                let (app, name) = (app.clone(), name.to_string());
                std::thread::spawn(move || {
                    if let Err(e) = launch_home(&app, &name) {
                        show(&app);
                        let _ = app.emit("backend-error", e);
                    }
                });
            }
        }
    }
}

fn launch_home(app: &AppHandle, account: &str) -> Result<(), backend::AppError> {
    let state = app.state::<Backend>();
    let dir = gui::check_launch(
        &state.config,
        &state.standard,
        account,
        LaunchAction::Run,
        None,
    )?;
    let home = state.home().ok_or(gui::GuiError::DirectoryMissing)?;
    launcher::open_terminal(home, &dir, account, LaunchAction::Run).map(|_| ())
}

/// `★ work · 5h 42% · cached`. A window whose reset has passed shows no
/// figure, and there is no relative age: the menu is not redrawn on a clock,
/// so "5m ago" would stay "5m ago" all afternoon.
fn label(a: &Account, now: u64) -> String {
    let marker = if a.is_default { "★ " } else { "" };
    let percent = a
        .usage
        .five_hour
        .as_ref()
        .filter(|w| !w.reset_passed(now))
        .map(|w| format!(" · 5h {:.0}%", w.percent))
        .unwrap_or_default();
    let source = match a.usage.source {
        UsageSource::Live => "live",
        UsageSource::Cached => "cached",
        UsageSource::NoCredentials => "no credentials",
        UsageSource::Unavailable => "no reading",
    };
    format!("{marker}{}{percent} · {source}", a.name)
}

pub fn update(app: &AppHandle, snapshot: &Snapshot) {
    let build = || -> tauri::Result<Menu<tauri::Wry>> {
        let menu = Menu::new(app)?;
        let item = |id: &str, text: &str, enabled: bool, accel: Option<&str>| {
            MenuItem::with_id(app, id, text, enabled, accel)
        };
        menu.append(&item(
            "show",
            "Open account manager",
            true,
            Some("CmdOrCtrl+Shift+A"),
        )?)?;
        let now = gui::now();
        for a in &snapshot.accounts {
            menu.append(&item(
                &format!("launch:{}", a.name),
                &label(a, now),
                a.available,
                None,
            )?)?;
        }
        menu.append(&item("bindings", "Project bindings…", true, None)?)?;
        menu.append(&item("refresh", "Refresh identity & usage", true, None)?)?;
        menu.append(&item("quit", "Quit", true, Some("CmdOrCtrl+Q"))?)?;
        Ok(menu)
    };
    if let (Some(tray), Ok(menu)) = (app.tray_by_id(TRAY), build()) {
        let _ = tray.set_menu(Some(menu));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claude_acc::gui::{IdentitySource, LockStatus, Usage, UsageWindow};

    fn account(source: UsageSource, resets_at: Option<u64>) -> Account {
        Account {
            name: "work".into(),
            email: None,
            plan: None,
            organization: None,
            is_default: true,
            available: true,
            identity_source: IdentitySource::Local,
            lock: LockStatus::Matched,
            usage: Usage {
                source,
                fetched_at: Some(100),
                five_hour: Some(UsageWindow {
                    percent: 41.6,
                    resets_at,
                }),
                seven_day: None,
            },
        }
    }

    #[test]
    fn a_reset_window_shows_no_figure_in_the_menu() {
        let running = account(UsageSource::Cached, Some(1_000));
        assert_eq!(label(&running, 999), "★ work · 5h 42% · cached");
        assert_eq!(label(&running, 1_000), "★ work · cached");
        assert_eq!(
            label(&account(UsageSource::Live, None), 5_000),
            "★ work · 5h 42% · live"
        );
    }
}
