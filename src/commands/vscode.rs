use crate::config::AppConfig;
use crate::i18n::{I18n, Msg};
use crate::vscode::{self, WrapperState};

/// Point every installed VS Code-family editor's
/// `claudeCode.claudeProcessWrapper` at our launcher, so the extension's
/// native UI resolves the account from the workspace folder like the
/// terminal already does.
pub fn install(config: &AppConfig, i18n: &I18n, force: bool) -> i32 {
    if cfg!(windows) {
        i18n.print(Msg::VscodeWindowsUnsupported);
        return 1;
    }

    let editors = vscode::detect_editors();
    if editors.is_empty() {
        i18n.print(Msg::VscodeNoEditors);
        return 1;
    }

    let bin = config
        .base_dir
        .join("bin")
        .join(super::install::binary_name());
    // Fall back to the running binary when nothing is installed yet, so
    // `vscode install` before `install` still produces a wrapper that works.
    let bin = if bin.is_file() {
        bin
    } else {
        std::env::current_exe().unwrap_or(bin)
    };

    let wrapper = match install_wrapper(config, &bin) {
        Ok(w) => w,
        Err(e) => {
            i18n.print(Msg::VscodeWrapperFailed(e.to_string()));
            return 1;
        }
    };

    let mut wrote = 0;
    let mut failed = false;
    for ed in &editors {
        match vscode::wrapper_state(&ed.settings, &wrapper) {
            WrapperState::Ours if !force => {
                i18n.print(Msg::VscodeAlready(ed.label.to_string()));
                continue;
            }
            WrapperState::Foreign(other) if !force => {
                i18n.print(Msg::VscodeForeign(ed.label.to_string(), other));
                failed = true;
                continue;
            }
            WrapperState::Unreadable => {
                i18n.print(Msg::VscodeUnreadable(
                    ed.label.to_string(),
                    ed.settings.display().to_string(),
                ));
                failed = true;
                continue;
            }
            _ => {}
        }
        match vscode::set_wrapper(&ed.settings, &wrapper) {
            Ok(true) => {
                i18n.print(Msg::VscodeConfigured(
                    ed.label.to_string(),
                    wrapper.display().to_string(),
                ));
                wrote += 1;
            }
            Ok(false) => {
                i18n.print(Msg::VscodeUnreadable(
                    ed.label.to_string(),
                    ed.settings.display().to_string(),
                ));
                failed = true;
            }
            Err(e) => {
                i18n.print(Msg::VscodeWriteFailed(ed.label.to_string(), e.to_string()));
                failed = true;
            }
        }
    }

    if wrote > 0 {
        i18n.print(Msg::VscodeSideEffects);
        i18n.print(Msg::VscodeRestartHint);
    }
    if failed { 1 } else { 0 }
}

/// Remove the setting again. The wrapper file is left in place — it is
/// inert once nothing points at it, and `install` would only rewrite it.
pub fn uninstall(config: &AppConfig, i18n: &I18n) -> i32 {
    let editors = vscode::detect_editors();
    if editors.is_empty() {
        i18n.print(Msg::VscodeNoEditors);
        return 1;
    }
    let wrapper = vscode::wrapper_path(&config.base_dir);

    let mut removed = 0;
    let mut failed = false;
    for ed in &editors {
        match vscode::wrapper_state(&ed.settings, &wrapper) {
            // Only ever remove our own. A path someone set by hand, or
            // another tool's, is theirs to remove.
            WrapperState::Foreign(other) => {
                i18n.print(Msg::VscodeForeignKept(ed.label.to_string(), other));
                continue;
            }
            // Report it as unreadable rather than letting `clear_wrapper`
            // surface the read error as a write failure.
            WrapperState::Unreadable => {
                i18n.print(Msg::VscodeUnreadable(
                    ed.label.to_string(),
                    ed.settings.display().to_string(),
                ));
                failed = true;
                continue;
            }
            _ => {}
        }
        match vscode::clear_wrapper(&ed.settings) {
            Ok(true) => {
                i18n.print(Msg::VscodeRemoved(ed.label.to_string()));
                removed += 1;
            }
            Ok(false) => i18n.print(Msg::VscodeNotConfigured(ed.label.to_string())),
            Err(e) => {
                i18n.print(Msg::VscodeWriteFailed(ed.label.to_string(), e.to_string()));
                failed = true;
            }
        }
    }

    if removed > 0 {
        i18n.print(Msg::VscodeRestartHint);
    }
    if failed { 1 } else { 0 }
}

/// What every installed editor currently points at.
pub fn status(config: &AppConfig, i18n: &I18n) -> i32 {
    let editors = vscode::detect_editors();
    if editors.is_empty() {
        i18n.print(Msg::VscodeNoEditors);
        return 1;
    }
    let wrapper = vscode::wrapper_path(&config.base_dir);

    i18n.print(Msg::VscodeStatusHeader);
    for ed in &editors {
        let line = match vscode::wrapper_state(&ed.settings, &wrapper) {
            WrapperState::Ours => i18n.msg(Msg::VscodeStateOurs),
            WrapperState::Unset => i18n.msg(Msg::VscodeStateUnset),
            WrapperState::Foreign(other) => i18n.msg(Msg::VscodeStateForeign(other)),
            WrapperState::Unreadable => i18n.msg(Msg::VscodeStateUnreadable),
        };
        println!("    {:<18} {}", ed.label, line);
    }
    if editors.iter().any(|e| {
        matches!(
            vscode::wrapper_state(&e.settings, &wrapper),
            WrapperState::Unset
        )
    }) {
        i18n.print(Msg::VscodeStatusHint);
    }
    0
}

/// Printed at the end of `claude-acc install` when an editor is installed
/// and not yet wired up. A hint, not an action: the setting is machine-scoped
/// and lives in someone else's editor config, so it is opted into.
pub fn print_install_hint(config: &AppConfig, i18n: &I18n) {
    if cfg!(windows) {
        return;
    }
    let wrapper = vscode::wrapper_path(&config.base_dir);
    let unconfigured: Vec<&'static str> = vscode::detect_editors()
        .iter()
        .filter(|e| {
            matches!(
                vscode::wrapper_state(&e.settings, &wrapper),
                WrapperState::Unset
            )
        })
        .map(|e| e.label)
        .collect();
    if unconfigured.is_empty() {
        return;
    }
    i18n.print(Msg::InstallVscodeHint(unconfigured.join(", ")));
}

#[cfg(not(windows))]
fn install_wrapper(
    config: &AppConfig,
    bin: &std::path::Path,
) -> std::io::Result<std::path::PathBuf> {
    vscode::install_wrapper(&config.base_dir, bin)
}

#[cfg(windows)]
fn install_wrapper(
    config: &AppConfig,
    _bin: &std::path::Path,
) -> std::io::Result<std::path::PathBuf> {
    // Unreachable: `install` returns early on Windows. Kept so the module
    // compiles there.
    Ok(vscode::wrapper_path(&config.base_dir))
}
