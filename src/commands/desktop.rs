use crate::config::{AppConfig, is_reserved_name, validate_name};
use crate::desktop;
use crate::i18n::{I18n, Msg};
use crate::identity;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

/// The installed app, or `None` with the reason already printed. Every
/// launching command needs the same answer.
fn require_app(i18n: &I18n) -> Option<PathBuf> {
    let app = desktop::find_app();
    if app.is_none() {
        // A Store install is present but unusable, which is a different
        // problem from not having the app at all — and saying so is the
        // whole point, since the alternative is a user wondering why a
        // clearly-installed app "isn't found".
        if desktop::packaged_install() {
            i18n.print(Msg::DesktopStorePackage);
        } else {
            i18n.print(Msg::DesktopAppNotFound);
        }
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
    let mut cmd = desktop::launch_command(app, profile);
    if !desktop::launch_detaches() {
        // The app itself is the child here, and it lives as long as its
        // window does — waiting would hang the terminal for the session.
        return match cmd.spawn() {
            Ok(_) => 0,
            Err(e) => {
                i18n.print(Msg::DesktopLaunchFailed(e.to_string()));
                1
            }
        };
    }
    // `open` returns as soon as the app is up, so waiting costs a moment and
    // buys a real exit status — spawning instead would report success even
    // when the app failed to start.
    match cmd.status() {
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

/// The profile a seed comes from: another managed profile with `--from`,
/// otherwise the app's own. `None` means the caller already printed why.
fn source_profile(
    config: &AppConfig,
    i18n: &I18n,
    from: Option<&str>,
) -> Option<(PathBuf, String)> {
    match from {
        Some(name) => {
            if !require_name(i18n, name) {
                return None;
            }
            if !desktop::profile_exists(config, name) {
                i18n.print(Msg::DesktopNotFound(name.to_string()));
                return None;
            }
            Some((desktop::profile_path(config, name), name.to_string()))
        }
        None => match desktop::standard_profile() {
            Some(path) => Some((path, desktop::standard_label().to_string())),
            None => {
                i18n.print(Msg::DesktopUnsupported);
                None
            }
        },
    }
}

/// Copy the MCP config into `profile`, reporting what happened. Shared by
/// `add --seed` and `clone-config`, which differ only in whether the profile
/// was just created.
fn seed_into(
    config: &AppConfig,
    i18n: &I18n,
    profile: &std::path::Path,
    from: Option<&str>,
    force: bool,
) -> i32 {
    let Some((source, label)) = source_profile(config, i18n, from) else {
        return 1;
    };

    let plan = desktop::plan_clone(
        source.join(desktop::CONFIG_FILE).is_file(),
        profile.join(desktop::CONFIG_FILE).is_file(),
        force,
    );
    match plan {
        desktop::ClonePlan::NoSource => {
            i18n.print(Msg::DesktopCloneNoSource(label));
            1
        }
        desktop::ClonePlan::Keep => {
            i18n.print(Msg::DesktopCloneKeep);
            1
        }
        desktop::ClonePlan::Copy => match desktop::clone_config(&source, profile) {
            Ok(_) => {
                i18n.print(Msg::DesktopCloneDone(label));
                i18n.print(Msg::DesktopCloneAuthNote);
                0
            }
            Err(e) => {
                i18n.print(Msg::DesktopCloneFailed(e.to_string()));
                1
            }
        },
    }
}

pub fn clone_config(
    config: &AppConfig,
    i18n: &I18n,
    name: &str,
    from: Option<&str>,
    force: bool,
) -> i32 {
    if !require_name(i18n, name) {
        return 1;
    }
    if !desktop::profile_exists(config, name) {
        i18n.print(Msg::DesktopNotFound(name.to_string()));
        return 1;
    }
    seed_into(
        config,
        i18n,
        &desktop::profile_path(config, name),
        from,
        force,
    )
}

/// Clone the sandbox images from another profile, so this one doesn't
/// re-download ten gigabytes of identical bytes.
#[cfg(target_os = "macos")]
pub fn clone_sandbox(
    config: &AppConfig,
    i18n: &I18n,
    name: &str,
    from: Option<&str>,
    force: bool,
) -> i32 {
    use crate::desktop_vm;

    if !require_name(i18n, name) {
        return 1;
    }
    if !desktop::profile_exists(config, name) {
        i18n.print(Msg::DesktopNotFound(name.to_string()));
        return 1;
    }
    let Some((source, label)) = source_profile(config, i18n, from) else {
        return 1;
    };

    let profile = desktop::profile_path(config, name);
    let src_bundle = desktop_vm::bundle_of(&source);
    let dest_bundle = desktop_vm::bundle_of(&profile);
    let plan = desktop_vm::plan(
        desktop_vm::images(&src_bundle).len(),
        desktop_vm::images(&dest_bundle).len(),
        // An unresolvable path is treated as "not the same filesystem": the
        // cautious answer, since the cost of being wrong is a real 10 GB copy.
        desktop_vm::same_device(&source, &profile).unwrap_or(false),
        force,
    );

    match plan {
        desktop_vm::SandboxPlan::NoSource => {
            i18n.print(Msg::DesktopSandboxNoSource(label));
            1
        }
        desktop_vm::SandboxPlan::Keep => {
            i18n.print(Msg::DesktopSandboxKeep);
            1
        }
        desktop_vm::SandboxPlan::WouldCopy => {
            i18n.print(Msg::DesktopSandboxWouldCopy);
            1
        }
        desktop_vm::SandboxPlan::Clone => match desktop_vm::clone_images(&src_bundle, &dest_bundle)
        {
            Ok(report) => {
                i18n.print(Msg::DesktopSandboxCloned(
                    report.files.to_string(),
                    crate::sessions::human_size(report.logical),
                    label,
                ));
                match report.on_disk {
                    Some(bytes) => {
                        i18n.print(Msg::DesktopSandboxCost(crate::sessions::human_size(bytes)))
                    }
                    None => i18n.print(Msg::DesktopSandboxCostUnknown),
                }
                i18n.print(Msg::DesktopSandboxUnverified);
                0
            }
            Err(e) => {
                i18n.print(Msg::DesktopSandboxFailed(e.to_string()));
                1
            }
        },
    }
}

pub fn add(config: &AppConfig, i18n: &I18n, name: &str, seed: bool) -> i32 {
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

    // A fresh profile has no config, so nothing here can be overwritten —
    // and a failed seed is not a reason to withhold the profile itself.
    if seed {
        seed_into(config, i18n, &profile, None, false);
    }

    i18n.print(Msg::DesktopSignInHint);
    i18n.print(Msg::DesktopSignInAloneWarning);
    i18n.print(Msg::DesktopDiskNote);
    i18n.print(Msg::DesktopDiskHint(name.to_string()));
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
    let mut any_unknown = false;
    for name in &profiles {
        let profile = desktop::profile_path(config, name);
        // Cached identity where we have one, otherwise just whether the
        // profile holds a credential. Nothing here reads the Keychain or the
        // network — `desktop usage` is where that happens.
        let suffix = identity_suffix(&profile);
        let state = if desktop::is_signed_in(&profile) {
            // The hint is about the *email*, so a uuid-only row still needs
            // it — that is precisely the row it exists for.
            any_unknown |= identity::read_cache(&profile)
                .and_then(|c| c.email)
                .is_none();
            i18n.msg(Msg::DesktopSignedIn)
        } else {
            i18n.msg(Msg::DesktopSignedOut)
        };
        println!("    {}{}  {}", name, suffix, state);
    }
    // The app's own profile, so the list reads as the full picture rather
    // than only the part this tool created.
    if let Some(standard) = desktop::standard_profile()
        && standard.is_dir()
    {
        println!(
            "    {}  {}",
            desktop::standard_label(),
            i18n.msg(Msg::DesktopStandard)
        );
    }
    // After the rows, so it reads as a footnote to the whole listing.
    if any_unknown {
        i18n.print(Msg::DesktopIdentityHint);
    }
    0
}

/// `"  <email>  Max 20x"` for a profile whose identity we know, `"  aa6c22d5-…"`
/// when only the plaintext uuid is available, `""` when neither.
///
/// The uuid fallback is deliberately shown truncated and unadorned: it says
/// "these two profiles are different accounts" without pretending to be an
/// identity anyone recognises.
fn identity_suffix(profile: &std::path::Path) -> String {
    let cached = crate::commands::usage::label_suffix(identity::read_cache(profile));
    if !cached.is_empty() {
        return cached;
    }
    match crate::desktop_auth::last_known_account_uuid(profile) {
        Some(uuid) => format!("  {}…", uuid.chars().take(8).collect::<String>()),
        None => String::new(),
    }
}

/// Identity and rate-limit usage for every profile, live.
///
/// Separate from `list` for the same reason `usage` is separate from `list`
/// on the CLI side: it needs the token, hence the network — and here also a
/// Keychain prompt, which no listing command should spring on anyone.
pub fn usage(config: &AppConfig, i18n: &I18n) -> i32 {
    let profiles = desktop::list_profiles(config).unwrap_or_default();
    if profiles.is_empty() {
        i18n.print(Msg::DesktopListEmpty);
        return 0;
    }
    if !desktop::identity_supported() {
        i18n.print(Msg::DesktopIdentityMacOnly);
        return 1;
    }

    // Said before the prompt appears, not after: an unexplained request for
    // the login keychain password is exactly what people should refuse.
    i18n.print(Msg::DesktopKeychainNote);
    println!();
    i18n.print(Msg::DesktopUsageHeader);

    for name in &profiles {
        let profile = desktop::profile_path(config, name);
        match crate::desktop_auth::profile_token(&profile) {
            crate::desktop_auth::TokenResult::Ok(token) => {
                // Refresh the cache first so the row can lead with the live
                // email rather than a stale one — same side effect `doctor`
                // has for CLI accounts.
                if let Some(fresh) = identity::fetch_profile(&token) {
                    let _ = identity::write_cache_at(
                        &profile.join(".account-info.json"),
                        &fresh,
                        &token,
                    );
                }
                println!("    {}{}", name, identity_suffix(&profile));
                match identity::fetch_usage(&token) {
                    Some(u) => crate::commands::usage::print_usage(&u, i18n),
                    None => println!("      {}", i18n.msg(Msg::DoctorOffline)),
                }
            }
            other => {
                println!("    {}{}", name, identity_suffix(&profile));
                println!("      {}", i18n.msg(reason(other)));
            }
        }
    }
    0
}

fn reason(result: crate::desktop_auth::TokenResult) -> Msg {
    use crate::desktop_auth::TokenResult;
    match result {
        TokenResult::NotSignedIn => Msg::DesktopNotSignedIn,
        TokenResult::NoKeychain => Msg::DesktopKeychainDenied,
        TokenResult::Unreadable => Msg::DesktopTokenUnreadable,
        TokenResult::Expired => Msg::DesktopTokenExpired,
        // `usage` handles the success case before calling this.
        TokenResult::Ok(_) => Msg::DesktopTokenUnreadable,
    }
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

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("cc-dtop-{}-{}", tag, std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_profile_we_know_nothing_about_gets_no_suffix() {
        let dir = scratch("suffix-none");
        assert_eq!(identity_suffix(&dir), "");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_plaintext_uuid_stands_in_until_the_identity_is_known() {
        // Enough to tell two profiles apart without any Keychain access.
        let dir = scratch("suffix-uuid");
        fs::write(
            dir.join("config.json"),
            r#"{"lastKnownAccountUuid":"aa6c22d5-f7d1-4ac1-bb29-22abc90481c1"}"#,
        )
        .unwrap();
        assert_eq!(identity_suffix(&dir), "  aa6c22d5…");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_cached_email_wins_over_the_uuid() {
        let dir = scratch("suffix-email");
        fs::write(
            dir.join("config.json"),
            r#"{"lastKnownAccountUuid":"aa6c22d5-f7d1-4ac1-bb29-22abc90481c1"}"#,
        )
        .unwrap();
        fs::write(
            dir.join(".account-info.json"),
            r#"{"email":"a@b.com","plan":"Max 20x"}"#,
        )
        .unwrap();
        assert_eq!(identity_suffix(&dir), "  <a@b.com>  Max 20x");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn every_token_failure_says_something_different() {
        use crate::desktop_auth::TokenResult;
        let i = i18n();
        let messages = [
            i.msg(reason(TokenResult::NotSignedIn)),
            i.msg(reason(TokenResult::NoKeychain)),
            i.msg(reason(TokenResult::Unreadable)),
            i.msg(reason(TokenResult::Expired)),
        ];
        for (n, m) in messages.iter().enumerate() {
            assert!(!m.is_empty());
            assert!(
                !messages[n + 1..].contains(m),
                "two failures print the same thing: {}",
                m
            );
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
