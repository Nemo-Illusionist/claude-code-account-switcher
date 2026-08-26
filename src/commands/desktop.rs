use crate::config::{AppConfig, is_reserved_name, validate_name};
use crate::desktop;
use crate::i18n::{I18n, Msg};
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

/// The app, once both "this platform is supported" and "the app is installed"
/// have been established. Every launching command needs the same two answers.
fn require_app(i18n: &I18n) -> Option<PathBuf> {
    if !desktop::supported() {
        i18n.print(Msg::DesktopUnsupported);
        return None;
    }
    let app = desktop::find_app();
    if app.is_none() {
        i18n.print(Msg::DesktopAppNotFound);
    }
    app
}

/// A managed profile name: valid, not the reserved `default` (the app's own
/// profile is not ours to manage).
fn require_name(i18n: &I18n, name: &str) -> bool {
    if is_reserved_name(name) {
        i18n.print(Msg::DesktopNoDefault);
        return false;
    }
    if !validate_name(name) {
        i18n.print(Msg::NameInvalid);
        return false;
    }
    true
}

fn launch(i18n: &I18n, app: &std::path::Path, profile: &std::path::Path) -> i32 {
    // `open` returns as soon as the app is up, so waiting costs a moment and
    // buys a real exit status — spawning instead would report success even
    // when the app failed to start.
    match desktop::launch_command(app, profile).status() {
        Ok(status) if status.success() => 0,
        Ok(status) => {
            i18n.print(Msg::DesktopLaunchFailed(status.to_string()));
            1
        }
        Err(e) => {
            i18n.print(Msg::DesktopLaunchFailed(e.to_string()));
            1
        }
    }
}

pub fn add(config: &AppConfig, i18n: &I18n, name: &str) -> i32 {
    if !require_name(i18n, name) {
        return 1;
    }
    if desktop::profile_exists(config, name) {
        i18n.print(Msg::DesktopExists(name.to_string()));
        return 1;
    }
    let Some(app) = require_app(i18n) else {
        return 1;
    };

    let profile = desktop::profile_path(config, name);
    if let Err(e) = fs::create_dir_all(&profile) {
        i18n.print(Msg::DesktopCreateFailed(e.to_string()));
        return 1;
    }

    i18n.print(Msg::DesktopCreated(name.to_string()));
    i18n.print(Msg::DesktopSignInHint);
    i18n.print(Msg::DesktopDiskNote);
    let code = launch(i18n, &app, &profile);
    if code == 0 {
        println!();
        i18n.print(Msg::DesktopHintRun(name.to_string()));
    }
    code
}

pub fn list(config: &AppConfig, i18n: &I18n) -> i32 {
    let profiles = desktop::list_profiles(config).unwrap_or_default();
    if profiles.is_empty() {
        i18n.print(Msg::DesktopListEmpty);
        return 0;
    }

    i18n.print(Msg::DesktopListHeader);
    for name in &profiles {
        let profile = desktop::profile_path(config, name);
        let state = if desktop::is_signed_in(&profile) {
            i18n.msg(Msg::DesktopSignedIn)
        } else {
            i18n.msg(Msg::DesktopSignedOut)
        };
        println!("    {}  {}", name, state);
    }

    // The app's own profile, so the list reads as the full picture rather
    // than only the part this tool created.
    if let Some(standard) = desktop::standard_profile()
        && standard.is_dir()
    {
        println!(
            "    ~/Library/…/Claude/  {}",
            i18n.msg(Msg::DesktopStandard)
        );
    }
    0
}

pub fn run(config: &AppConfig, i18n: &I18n, name: &str) -> i32 {
    if !require_name(i18n, name) {
        return 1;
    }
    if !desktop::profile_exists(config, name) {
        i18n.print(Msg::DesktopNotFound(name.to_string()));
        return 1;
    }
    let Some(app) = require_app(i18n) else {
        return 1;
    };

    i18n.print(Msg::DesktopLaunching(name.to_string()));
    launch(i18n, &app, &desktop::profile_path(config, name))
}

pub fn remove(config: &AppConfig, i18n: &I18n, name: &str, force: bool) -> i32 {
    if !require_name(i18n, name) {
        return 1;
    }
    let profile = desktop::profile_path(config, name);
    if !profile.is_dir() {
        i18n.print(Msg::DesktopNotFound(name.to_string()));
        return 1;
    }

    if !force {
        i18n.print(Msg::DesktopRemoveWarn(name.to_string()));
        print!("{}", i18n.msg(Msg::DesktopRemoveConfirm(name.to_string())));
        io::stdout().flush().ok();
        let mut reply = String::new();
        if io::stdin().read_line(&mut reply).is_err() {
            i18n.print(Msg::DesktopRemoveCancelled);
            return 1;
        }
        let reply = reply.trim().to_lowercase();
        if !reply.starts_with('y') && !reply.starts_with('д') {
            i18n.print(Msg::DesktopRemoveCancelled);
            return 1;
        }
    }

    if let Err(e) = fs::remove_dir_all(&profile) {
        i18n.print(Msg::DesktopRemoveFailed(e.to_string()));
        return 1;
    }
    i18n.print(Msg::DesktopRemoved(name.to_string()));
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn i18n() -> I18n {
        I18n {
            lang: crate::i18n::Lang::En,
        }
    }

    #[test]
    fn default_is_rejected_as_a_profile_name() {
        // The app's own profile is not managed by this tool — there is
        // nothing to add, launch, or delete for it.
        assert!(!require_name(&i18n(), "default"));
    }

    #[test]
    fn invalid_names_are_rejected() {
        assert!(!require_name(&i18n(), "../etc"));
        assert!(!require_name(&i18n(), "a b"));
        assert!(!require_name(&i18n(), ""));
    }

    #[test]
    fn ordinary_names_are_accepted() {
        assert!(require_name(&i18n(), "work"));
        assert!(require_name(&i18n(), "personal-2"));
    }
}
