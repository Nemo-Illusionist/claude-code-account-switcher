//! The menu bar: account entries that open Claude, plus refresh and
//! navigation. Refresh and launch run here, on worker threads, so the menu
//! works with the window closed and never blocks the main thread.
use crate::backend::{self, Backend};
use crate::copy::{LANG, Text, text};
use crate::launcher;
use claude_acc::gui::{self, Account, LaunchAction, Snapshot};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager};

const TRAY: &str = "accounts";

pub fn install(app: &AppHandle) -> tauri::Result<()> {
    let tray = TrayIconBuilder::with_id(TRAY)
        .tooltip(text(Text::TrayTooltip, LANG))
        .on_menu_event(|app, event| on_click(app, event.id().as_ref()));
    // The icon is bundled, so missing means a damaged bundle. A text title
    // keeps the menu reachable instead of ending the app over it.
    let tray = match app.default_window_icon() {
        Some(icon) => tray.icon(icon.clone()),
        None => {
            eprintln!("claude-acc-manager: {}", text(Text::IconMissing, LANG));
            tray.title(text(Text::TrayTitle, LANG))
        }
    };
    tray.build(app)?;
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

/// The figure the window shows for a reading: half rounds up, as its
/// `Math.round` does. `{:.0}` rounds half to even and `f64::round` half away
/// from zero, so either would put 98% here beside the window's 99%.
fn shown_percent(percent: f64) -> i64 {
    (percent + 0.5).floor() as i64
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
        .map(|w| {
            format!(
                " · {}",
                text(Text::FiveHour(shown_percent(w.percent)), LANG)
            )
        })
        .unwrap_or_default();
    let source = text(Text::Source(a.usage.source), LANG);
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
            &text(Text::OpenManager, LANG),
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
        menu.append(&item(
            "bindings",
            &text(Text::ProjectBindings, LANG),
            true,
            None,
        )?)?;
        menu.append(&item("refresh", &text(Text::Refresh, LANG), true, None)?)?;
        menu.append(&item(
            "quit",
            &text(Text::Quit, LANG),
            true,
            Some("CmdOrCtrl+Q"),
        )?)?;
        Ok(menu)
    };
    if let (Some(tray), Ok(menu)) = (app.tray_by_id(TRAY), build()) {
        let _ = tray.set_menu(Some(menu));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claude_acc::gui::{IdentitySource, LockStatus, Usage, UsageSource, UsageWindow};

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
    fn the_menu_rounds_a_reading_as_the_window_does() {
        // Regression: `{:.0}` rounds half to even, so 98.5 read 98% here and
        // 99% in the window. Expected values are Node's `Math.round`.
        for (percent, shown) in [
            (0.5, 1),
            (2.5, 3),
            (98.5, 99),
            (99.5, 100),
            (120.5, 121),
            (-5.5, -5),
        ] {
            assert_eq!(shown_percent(percent), shown, "{percent}");
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
