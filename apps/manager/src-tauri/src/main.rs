#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod backend;
mod commands;
mod copy;
mod launcher;
mod tray;

use claude_acc::{config::AppConfig, identity};
use copy::{LANG, Text, text};

fn main() {
    // Checked before `AppConfig::new()`, which would panic on the same
    // condition with no window open and nothing for the user to read.
    let Some(standard) = identity::standard_token_dir() else {
        fail(&text(Text::NoHome, LANG));
    };
    let started = tauri::Builder::default()
        .manage(backend::Backend::new(AppConfig::new(), standard))
        .invoke_handler(tauri::generate_handler![
            commands::snapshot,
            commands::set_default,
            commands::remove_account,
            commands::import_account,
            commands::pin_account,
            commands::clone_settings,
            commands::install_statusline,
            commands::list_bindings,
            commands::set_binding,
            commands::list_sessions,
            commands::copy_session,
            commands::resume_hook,
            commands::set_resume_hook,
            commands::vscode_editors,
            commands::vscode_install,
            commands::vscode_uninstall,
            commands::open_terminal,
            commands::installation,
            commands::check_update,
            commands::choose_directory,
        ])
        .setup(|app| {
            tray::install(app.handle())?;
            // Local records only: fills the menu before the window asks.
            let _ = backend::refresh(app.handle(), false);
            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing hides the window; the menu bar keeps running.
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .run(tauri::generate_context!());
    if let Err(e) = started {
        fail(&text(Text::StartFailed(&e.to_string()), LANG));
    }
}

/// End a startup that cannot continue, in a way the user can read: on stderr
/// for a launch from a terminal, and in a dialog for one from Finder, where
/// nothing else would be seen.
fn fail(message: &str) -> ! {
    eprintln!("claude-acc-manager: {message}");
    rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Error)
        .set_title(text(Text::ProductName, LANG))
        .set_description(message)
        .set_buttons(rfd::MessageButtons::Ok)
        .show();
    std::process::exit(1);
}
