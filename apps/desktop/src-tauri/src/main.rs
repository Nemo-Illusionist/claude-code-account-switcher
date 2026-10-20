#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod backend;
mod commands;
mod launcher;
mod tray;

use claude_acc::{config::AppConfig, identity};

fn main() {
    let standard = identity::standard_token_dir().expect("home directory");
    tauri::Builder::default()
        .manage(backend::Backend::new(AppConfig::new(), standard))
        .invoke_handler(tauri::generate_handler![
            commands::snapshot,
            commands::set_default,
            commands::list_bindings,
            commands::set_binding,
            commands::list_sessions,
            commands::open_terminal,
            commands::installation,
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
        .run(tauri::generate_context!())
        .expect("desktop runtime failed");
}
