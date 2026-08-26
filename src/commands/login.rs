use crate::config::{AppConfig, validate_name};
use crate::environment::strip_claude_auth_env;
use crate::i18n::{I18n, Msg};
use crate::identity;
use std::path::Path;
use std::process::Command;

/// Builds the `claude auth login` invocation. `acc_dir: None` means the
/// standard `~/.claude` account: clears any inherited CLAUDE_CONFIG_DIR and
/// sets CLAUDE_ACC_RUN_DEFAULT so claude-acc's IDE wrapper doesn't re-derive
/// one from $PWD — same reasoning as `run default` (see commands::run).
/// Also strips ANTHROPIC_API_KEY / ANTHROPIC_AUTH_TOKEN /
/// CLAUDE_CODE_OAUTH_TOKEN / AWS_BEARER_TOKEN_BEDROCK — a leaked one of these
/// can make the login skip the OAuth flow entirely, or auth a different
/// identity than intended.
fn build_login_command(acc_dir: Option<&Path>) -> Command {
    let mut cmd = Command::new("claude");
    cmd.args(["auth", "login"]);
    match acc_dir {
        Some(dir) => {
            cmd.env("CLAUDE_CONFIG_DIR", dir);
            cmd.env_remove("CLAUDE_ACC_RUN_DEFAULT");
        }
        None => {
            cmd.env_remove("CLAUDE_CONFIG_DIR");
            cmd.env("CLAUDE_ACC_RUN_DEFAULT", "1");
        }
    }
    strip_claude_auth_env(&mut cmd);
    cmd
}

pub fn run(config: &AppConfig, i18n: &I18n, name: &str) {
    if name == "default" {
        login_default(i18n);
        return;
    }

    if !validate_name(name) {
        i18n.print(Msg::NameInvalid);
        std::process::exit(1);
    }

    let acc_dir = config.account_path(name);
    if !acc_dir.is_dir() {
        i18n.print(Msg::LoginNotFound(name.to_string()));
        std::process::exit(1);
    }

    i18n.print(Msg::LoginStart(name.to_string()));
    // See identity::snapshot_side_effect_keychain: `claude auth login` can
    // clobber the standard account's own Keychain entries as a side effect
    // even when scoped to acc_dir's CLAUDE_CONFIG_DIR.
    let keychain_snapshot = identity::snapshot_side_effect_keychain();
    build_login_command(Some(&acc_dir))
        .status()
        .expect("Failed to run claude auth login");
    identity::restore_side_effect_keychain(keychain_snapshot);

    i18n.print(Msg::LoginDone);
}

/// Re-login the standard `~/.claude` account. No keychain snapshot/restore
/// here — unlike logging into a *different* account, this login is meant to
/// change the standard account's own credentials.
fn login_default(i18n: &I18n) {
    i18n.print(Msg::LoginStart("default".to_string()));
    build_login_command(None)
        .status()
        .expect("Failed to run claude auth login");

    i18n.print(Msg::LoginDone);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn named_account_sets_config_dir_and_clears_run_default_marker() {
        let dir = Path::new("/tmp/some-account");
        let cmd = build_login_command(Some(dir));

        let config_dir = cmd
            .get_envs()
            .find(|(k, _)| *k == std::ffi::OsStr::new("CLAUDE_CONFIG_DIR"));
        assert_eq!(
            config_dir,
            Some((
                std::ffi::OsStr::new("CLAUDE_CONFIG_DIR"),
                Some(std::ffi::OsStr::new("/tmp/some-account"))
            ))
        );

        let marker = cmd
            .get_envs()
            .find(|(k, _)| *k == std::ffi::OsStr::new("CLAUDE_ACC_RUN_DEFAULT"));
        assert_eq!(
            marker,
            Some((std::ffi::OsStr::new("CLAUDE_ACC_RUN_DEFAULT"), None))
        );
    }

    #[test]
    fn default_account_clears_config_dir_and_sets_run_default_marker() {
        let cmd = build_login_command(None);

        let config_dir = cmd
            .get_envs()
            .find(|(k, _)| *k == std::ffi::OsStr::new("CLAUDE_CONFIG_DIR"));
        assert_eq!(
            config_dir,
            Some((std::ffi::OsStr::new("CLAUDE_CONFIG_DIR"), None))
        );

        let marker = cmd
            .get_envs()
            .find(|(k, _)| *k == std::ffi::OsStr::new("CLAUDE_ACC_RUN_DEFAULT"));
        assert_eq!(
            marker,
            Some((
                std::ffi::OsStr::new("CLAUDE_ACC_RUN_DEFAULT"),
                Some(std::ffi::OsStr::new("1"))
            ))
        );
    }

    #[test]
    fn strips_auth_env_vars_for_both_default_and_named_account() {
        for acc_dir in [None, Some(Path::new("/tmp/some-account"))] {
            let cmd = build_login_command(acc_dir);
            for var in crate::environment::CLAUDE_AUTH_ENV_VARS {
                let removed = cmd
                    .get_envs()
                    .find(|(k, _)| *k == std::ffi::OsStr::new(*var));
                assert_eq!(
                    removed,
                    Some((std::ffi::OsStr::new(*var), None)),
                    "{var} not stripped for acc_dir={acc_dir:?}"
                );
            }
        }
    }

    #[test]
    fn login_command_uses_claude_auth_login_args() {
        let cmd = build_login_command(None);
        let args: Vec<_> = cmd.get_args().collect();
        assert_eq!(args, vec!["auth", "login"]);
    }
}
