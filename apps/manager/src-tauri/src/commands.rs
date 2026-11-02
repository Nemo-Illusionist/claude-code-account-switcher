//! The named operations the webview may invoke. Each one validates in
//! `claude_acc::gui` and runs off the main thread.
use crate::backend::{self, AppError, Backend, Revisioned};
use crate::copy::{LANG, Text, text};
use crate::launcher::{self, Installation, Launch};
use claude_acc::desktop;
use claude_acc::gui::{
    self, Bindings, DesktopAction, DesktopProfiles, Editor, LaunchAction, RuntimeCloned, Session,
    UpdateCheck,
};
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
pub async fn import_account(
    app: AppHandle,
    account: String,
    source: String,
    move_into: bool,
) -> Result<Revisioned, AppError> {
    blocking(app, move |app| {
        backend::import_account(app, &account, &source, move_into)
    })
    .await
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
pub async fn clone_settings(app: AppHandle, account: String) -> Result<Vec<String>, AppError> {
    blocking(app, move |app| {
        let b = app.state::<Backend>();
        Ok(gui::clone_settings(&b.config, &b.standard, &account)?)
    })
    .await
}

#[tauri::command]
pub async fn install_statusline(
    app: AppHandle,
    account: String,
    replace: bool,
) -> Result<(), AppError> {
    blocking(app, move |app| {
        let b = app.state::<Backend>();
        Ok(gui::install_statusline(
            &b.config,
            &b.standard,
            &account,
            replace,
        )?)
    })
    .await
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

/// Copy a session, then list sessions again: the copy changes which ids are
/// held in more than one account.
#[tauri::command]
pub async fn copy_session(
    app: AppHandle,
    from: String,
    project: String,
    id: String,
    to: String,
    replace: bool,
) -> Result<Vec<Session>, AppError> {
    blocking(app, move |app| {
        let b = app.state::<Backend>();
        gui::copy_session(&b.config, &b.standard, &from, &project, &id, &to, replace)?;
        Ok(gui::sessions(&b.config, &b.standard)?)
    })
    .await
}

#[tauri::command]
pub async fn resume_hook(app: AppHandle) -> Result<bool, AppError> {
    blocking(app, |app| {
        Ok(gui::resume_hook(&app.state::<Backend>().config)?)
    })
    .await
}

/// Store the setting, then read it back, so the window shows what is stored.
#[tauri::command]
pub async fn set_resume_hook(app: AppHandle, on: bool) -> Result<bool, AppError> {
    blocking(app, move |app| {
        let config = &app.state::<Backend>().config;
        gui::set_resume_hook(config, on)?;
        Ok(gui::resume_hook(config)?)
    })
    .await
}

#[tauri::command]
pub async fn vscode_editors(app: AppHandle) -> Result<Vec<Editor>, AppError> {
    blocking(app, |app| {
        Ok(gui::vscode_editors(&app.state::<Backend>().config))
    })
    .await
}

#[tauri::command]
pub async fn vscode_install(app: AppHandle, force: bool) -> Result<Vec<Editor>, AppError> {
    blocking(app, move |app| {
        Ok(gui::vscode_install(&app.state::<Backend>().config, force)?)
    })
    .await
}

#[tauri::command]
pub async fn vscode_uninstall(app: AppHandle) -> Result<Vec<Editor>, AppError> {
    blocking(app, |app| {
        Ok(gui::vscode_uninstall(&app.state::<Backend>().config)?)
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

fn desktop_list(app: &AppHandle) -> Result<DesktopProfiles, AppError> {
    let b = app.state::<Backend>();
    let app_profile = desktop::standard_profile();
    Ok(gui::desktop_profiles(
        &b.config,
        &b.standard,
        app_profile.as_deref(),
    )?)
}

#[tauri::command]
pub async fn desktop_profiles(app: AppHandle) -> Result<DesktopProfiles, AppError> {
    blocking(app, desktop_list).await
}

/// Open Claude Desktop on a profile, adding it first for an add. The app is
/// looked for before anything is created, as in the CLI.
#[tauri::command]
pub async fn desktop_open(
    app: AppHandle,
    account: String,
    action: DesktopAction,
    force: bool,
) -> Result<DesktopProfiles, AppError> {
    blocking(app, move |app| {
        let claude = gui::desktop_app()?;
        let profile = gui::desktop_prepare(
            &app.state::<Backend>().config,
            desktop::standard_profile().as_deref(),
            &account,
            action,
            force,
            &desktop::running_instances(),
        )?;
        gui::desktop_launch(&claude, &profile)?;
        desktop_list(app)
    })
    .await
}

#[tauri::command]
pub async fn desktop_clone_config(
    app: AppHandle,
    account: String,
    from: Option<String>,
    force: bool,
) -> Result<(), AppError> {
    blocking(app, move |app| {
        Ok(gui::desktop_clone_config(
            &app.state::<Backend>().config,
            desktop::standard_profile().as_deref(),
            &account,
            from.as_deref(),
            force,
        )?)
    })
    .await
}

#[tauri::command]
pub async fn desktop_clone_runtime(
    app: AppHandle,
    account: String,
    from: Option<String>,
    force: bool,
) -> Result<RuntimeCloned, AppError> {
    blocking(app, move |app| {
        Ok(gui::desktop_clone_runtime(
            &app.state::<Backend>().config,
            desktop::standard_profile().as_deref(),
            &account,
            from.as_deref(),
            force,
            &desktop::running_instances(),
        )?)
    })
    .await
}

#[tauri::command]
pub async fn desktop_remove(app: AppHandle, account: String) -> Result<DesktopProfiles, AppError> {
    blocking(app, move |app| {
        let config = &app.state::<Backend>().config;
        gui::desktop_remove(config, &account, &desktop::running_instances())?;
        desktop_list(app)
    })
    .await
}

#[tauri::command]
pub async fn installation(app: AppHandle) -> Result<Installation, AppError> {
    blocking(app, |app| Ok(app.state::<Backend>().installation())).await
}

/// Compares with the versions Details shows, read again for each check.
#[tauri::command]
pub async fn check_update(app: AppHandle) -> Result<UpdateCheck, AppError> {
    blocking(app, |app| {
        let installation = app.state::<Backend>().fresh_installation();
        Ok(gui::check_update(
            &installation.bundled_version,
            installation.installed_version.as_deref(),
        )?)
    })
    .await
}

#[tauri::command]
pub async fn choose_directory() -> Option<String> {
    rfd::AsyncFileDialog::new()
        .set_title(text(Text::ChooseDirectory, LANG))
        .pick_folder()
        .await
        .map(|p| p.path().to_string_lossy().into_owned())
}
