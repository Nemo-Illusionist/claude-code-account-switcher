use crate::config::AppConfig;
use crate::i18n::{I18n, Msg};
use crate::vscode::{self, WrapperState};

/// What `install` does with one editor, given what its settings.json says.
/// Split out from the loop so the policy can be checked without a
/// filesystem, an editor, or a terminal to print to.
#[derive(Debug, PartialEq)]
enum InstallAction {
    Write,
    AlreadySetUp,
    /// Someone else's launcher. Replaced only with `--force`.
    RefuseForeign(String),
    /// Never overwritten, `--force` or not: forcing is for replacing a
    /// wrapper, not for writing over a file we could not read.
    RefuseUnreadable,
}

fn plan_install(state: &WrapperState, force: bool) -> InstallAction {
    match state {
        WrapperState::Unreadable => InstallAction::RefuseUnreadable,
        _ if force => InstallAction::Write,
        WrapperState::Ours => InstallAction::AlreadySetUp,
        WrapperState::Foreign(other) => InstallAction::RefuseForeign(other.clone()),
        WrapperState::Unset => InstallAction::Write,
    }
}

/// What `uninstall` does with one editor. There is no `--force` here: a
/// launcher that isn't ours is not ours to remove.
#[derive(Debug, PartialEq)]
enum UninstallAction {
    Clear,
    KeepForeign(String),
    RefuseUnreadable,
}

fn plan_uninstall(state: &WrapperState) -> UninstallAction {
    match state {
        WrapperState::Foreign(other) => UninstallAction::KeepForeign(other.clone()),
        // Report it as unreadable rather than letting `clear_wrapper`
        // surface a read error as a write failure.
        WrapperState::Unreadable => UninstallAction::RefuseUnreadable,
        WrapperState::Ours | WrapperState::Unset => UninstallAction::Clear,
    }
}

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
        match plan_install(&vscode::wrapper_state(&ed.settings, &wrapper), force) {
            InstallAction::AlreadySetUp => {
                i18n.print(Msg::VscodeAlready(ed.label.to_string()));
                continue;
            }
            InstallAction::RefuseForeign(other) => {
                i18n.print(Msg::VscodeForeign(ed.label.to_string(), other));
                failed = true;
                continue;
            }
            InstallAction::RefuseUnreadable => {
                i18n.print(Msg::VscodeUnreadable(
                    ed.label.to_string(),
                    ed.settings.display().to_string(),
                ));
                failed = true;
                continue;
            }
            InstallAction::Write => {}
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

    for ed in &editors {
        report_profiles(i18n, ed);
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
        match plan_uninstall(&vscode::wrapper_state(&ed.settings, &wrapper)) {
            UninstallAction::KeepForeign(other) => {
                i18n.print(Msg::VscodeForeignKept(ed.label.to_string(), other));
                continue;
            }
            UninstallAction::RefuseUnreadable => {
                i18n.print(Msg::VscodeUnreadable(
                    ed.label.to_string(),
                    ed.settings.display().to_string(),
                ));
                failed = true;
                continue;
            }
            UninstallAction::Clear => {}
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
    for ed in &editors {
        report_profiles(i18n, ed);
    }
    // Don't point at `vscode install` on Windows, where it refuses. Say why
    // instead. (`uninstall` stays available there — taking the setting back
    // out has to work wherever it can be set.)
    if cfg!(windows) {
        i18n.print(Msg::VscodeWindowsUnsupported);
    } else if editors.iter().any(|e| {
        matches!(
            vscode::wrapper_state(&e.settings, &wrapper),
            WrapperState::Unset
        )
    }) {
        i18n.print(Msg::VscodeStatusHint);
    }
    0
}

/// Say when an editor has profiles beyond the default one. The setting is
/// per-profile, we only write the default profile's file, and a window on
/// another profile would go on ignoring the account with nothing to explain
/// why.
fn report_profiles(i18n: &I18n, ed: &vscode::Editor) {
    let names = vscode::extra_profiles(ed.dir);
    if !names.is_empty() {
        i18n.print(Msg::VscodeProfiles(ed.label.to_string(), names.join(", ")));
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn foreign() -> WrapperState {
        WrapperState::Foreign("/opt/other/launcher".to_string())
    }

    #[test]
    fn install_writes_where_nothing_is_set_and_skips_what_is_already_ours() {
        assert_eq!(
            plan_install(&WrapperState::Unset, false),
            InstallAction::Write
        );
        assert_eq!(
            plan_install(&WrapperState::Ours, false),
            InstallAction::AlreadySetUp
        );
    }

    #[test]
    fn install_refuses_a_foreign_wrapper_until_forced() {
        // Regression guard: inverting this condition would silently replace
        // another tool's launcher and break it, with nothing said.
        assert_eq!(
            plan_install(&foreign(), false),
            InstallAction::RefuseForeign("/opt/other/launcher".to_string())
        );
        assert_eq!(plan_install(&foreign(), true), InstallAction::Write);
    }

    #[test]
    fn force_rewrites_our_own_wrapper_rather_than_reporting_it_set_up() {
        // `--force` after the wrapper path moved has to actually rewrite it.
        assert_eq!(
            plan_install(&WrapperState::Ours, true),
            InstallAction::Write
        );
    }

    #[test]
    fn force_never_writes_over_a_file_that_could_not_be_read() {
        // Forcing replaces a wrapper. It is not permission to overwrite a
        // settings.json we failed to parse or failed to read at all — that
        // is how the whole file gets lost.
        for force in [false, true] {
            assert_eq!(
                plan_install(&WrapperState::Unreadable, force),
                InstallAction::RefuseUnreadable,
                "force={force}"
            );
        }
    }

    #[test]
    fn uninstall_clears_ours_and_does_nothing_where_nothing_is_set() {
        assert_eq!(plan_uninstall(&WrapperState::Ours), UninstallAction::Clear);
        assert_eq!(plan_uninstall(&WrapperState::Unset), UninstallAction::Clear);
    }

    #[test]
    fn uninstall_leaves_a_foreign_wrapper_alone() {
        assert_eq!(
            plan_uninstall(&foreign()),
            UninstallAction::KeepForeign("/opt/other/launcher".to_string())
        );
    }

    #[test]
    fn uninstall_reports_an_unreadable_file_instead_of_a_write_failure() {
        assert_eq!(
            plan_uninstall(&WrapperState::Unreadable),
            UninstallAction::RefuseUnreadable
        );
    }
}
