//! The named operations the webview may invoke. Each one validates in
//! `claude_acc::gui` and runs off the main thread.
use crate::backend::{self, AppError, Backend, Revisioned};
use crate::copy::{LANG, Text, text};
use crate::launcher::{self, Installation, Launch};
use claude_acc::gui::{self, Bindings, LaunchAction, Session};
use tauri::{AppHandle, Manager};

async fn blocking<T: Send + 'static>(
    app: AppHandle,
    work: impl FnOnce(&AppHandle) -> Result<T, AppError> + Send + 'static,
) -> Result<T, AppError> {
    tauri::async_runtime::spawn_blocking(move || work(&app))
        .await
        .map_err(|_| AppError::BackendFailed)?
}

#[tauri::command]
pub async fn snapshot(app: AppHandle, live: bool) -> Result<Revisioned, AppError> {
    blocking(app, move |app| backend::refresh(app, live)).await
}

#[tauri::command]
pub async fn set_default(app: AppHandle, account: String) -> Result<Revisioned, AppError> {
    blocking(app, move |app| backend::set_default(app, &account)).await
}

#[tauri::command]
pub async fn remove_account(app: AppHandle, account: String) -> Result<Revisioned, AppError> {
    blocking(app, move |app| backend::remove_account(app, &account)).await
}

#[tauri::command]
pub async fn pin_account(
    app: AppHandle,
    account: String,
    force: bool,
) -> Result<Revisioned, AppError> {
    blocking(app, move |app| backend::pin_account(app, &account, force)).await
}

#[tauri::command]
pub async fn list_bindings(
    app: AppHandle,
    directory: Option<String>,
) -> Result<Bindings, AppError> {
    blocking(app, move |app| {
        let b = app.state::<Backend>();
        Ok(gui::bindings(&b.config, directory.as_deref())?)
    })
    .await
}

#[tauri::command]
pub async fn set_binding(
    app: AppHandle,
    directory: String,
    account: Option<String>,
) -> Result<(), AppError> {
    blocking(app, move |app| {
        let b = app.state::<Backend>();
        Ok(gui::change_binding(
            &b.config,
            &b.standard,
            &directory,
            account.as_deref(),
        )?)
    })
    .await
}

#[tauri::command]
pub async fn list_sessions(app: AppHandle) -> Result<Vec<Session>, AppError> {
    blocking(app, |app| {
        let b = app.state::<Backend>();
        Ok(gui::sessions(&b.config, &b.standard)?)
    })
    .await
}

#[tauri::command]
pub async fn open_terminal(
    app: AppHandle,
    account: String,
    directory: Option<String>,
    action: LaunchAction,
) -> Result<Launch, AppError> {
    blocking(app, move |app| {
        let b = app.state::<Backend>();
        let dir = gui::check_launch(
            &b.config,
            &b.standard,
            &account,
            action,
            directory.as_deref(),
        )?;
        let home = b.home().ok_or(gui::GuiError::DirectoryMissing)?;
        launcher::open_terminal(home, &dir, &account, action)
    })
    .await
}

#[tauri::command]
pub async fn installation(app: AppHandle) -> Result<Installation, AppError> {
    blocking(app, |app| Ok(app.state::<Backend>().installation())).await
}

#[tauri::command]
pub async fn choose_directory() -> Option<String> {
    rfd::AsyncFileDialog::new()
        .set_title(text(Text::ChooseDirectory, LANG))
        .pick_folder()
        .await
        .map(|p| p.path().to_string_lossy().into_owned())
}
