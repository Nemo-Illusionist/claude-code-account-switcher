//! The named operations the webview may invoke. Each one validates in
//! `claude_acc::gui` and runs off the main thread.
use crate::backend::{self, AppError, Backend};
use crate::launcher::{self, Installation, Launch};
use claude_acc::gui::{self, Bindings, LaunchAction, Session, Snapshot};
use std::sync::OnceLock;
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
pub async fn snapshot(app: AppHandle, live: bool) -> Result<Snapshot, AppError> {
    blocking(app, move |app| backend::refresh(app, live)).await
}

#[tauri::command]
pub async fn set_default(app: AppHandle, account: String) -> Result<Snapshot, AppError> {
    blocking(app, move |app| backend::set_default(app, &account)).await
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
    static CACHE: OnceLock<Installation> = OnceLock::new();
    blocking(app, |app| {
        let base = &app.state::<Backend>().config.base_dir;
        Ok(CACHE.get_or_init(|| launcher::installation(base)).clone())
    })
    .await
}

#[tauri::command]
pub async fn choose_directory() -> Option<String> {
    rfd::AsyncFileDialog::new()
        .set_title("Choose project directory")
        .pick_folder()
        .await
        .map(|p| p.path().to_string_lossy().into_owned())
}
