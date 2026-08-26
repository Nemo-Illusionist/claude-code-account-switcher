use crate::config::{AppConfig, validate_name};
use crate::environment::strip_claude_auth_env;
use crate::i18n::{I18n, Msg};
use crate::ide;
use crate::identity;
use crate::seed;
use std::fs;
use std::path::Path;
use std::process::Command;

/// Builds the `claude auth login` invocation scoped to `acc_dir`. Also
/// strips ANTHROPIC_API_KEY / ANTHROPIC_AUTH_TOKEN / CLAUDE_CODE_OAUTH_TOKEN /
/// AWS_BEARER_TOKEN_BEDROCK — a leaked one of these can make the login skip
/// the OAuth flow entirely, or auth a different identity than acc_dir intends.
fn build_login_command(acc_dir: &Path) -> Command {
    let mut cmd = Command::new("claude");
    cmd.args(["auth", "login"])
        .env("CLAUDE_CONFIG_DIR", acc_dir);
    strip_claude_auth_env(&mut cmd);
    cmd
}

pub fn run(config: &AppConfig, i18n: &I18n, name: &str, seed_from_default: bool) {
    if name == "default" {
        i18n.print(Msg::ReservedName(name.to_string()));
        std::process::exit(1);
    }

    if !validate_name(name) {
        i18n.print(Msg::NameInvalid);
        std::process::exit(1);
    }

    let acc_dir = config.account_path(name);
    if acc_dir.is_dir() {
        i18n.print(Msg::AddExists(name.to_string()));
        std::process::exit(1);
    }

    fs::create_dir_all(&acc_dir).expect("Failed to create account directory");
    ide::ensure_account_symlink(&acc_dir).ok();

    // Seed before printing AddCreated, so the copy report is logically
    // attached to "what the new account got". Errors here are non-fatal —
    // an empty account dir is still usable.
    if seed_from_default {
        match seed::copy_user_config(&acc_dir) {
            Ok(report) if report.is_empty() => {
                i18n.print(Msg::SeedNothingToCopy);
            }
            Ok(report) => {
                for entry in &report.copied {
                    i18n.print(Msg::SeedCopied(entry.clone()));
                }
            }
            Err(e) => eprintln!("seed: {}", e),
        }
    }

    i18n.print(Msg::AddCreated(name.to_string()));

    // `claude auth login` can write to the standard account's own Keychain
    // entries as a side effect, even though this login is scoped to
    // acc_dir's CLAUDE_CONFIG_DIR — snapshot/restore undoes that collateral.
    // See identity::snapshot_side_effect_keychain for why.
    let keychain_snapshot = identity::snapshot_side_effect_keychain();
    build_login_command(&acc_dir)
        .status()
        .expect("Failed to run claude auth login");
    identity::restore_side_effect_keychain(keychain_snapshot);

    println!();
    i18n.print(Msg::AddDone);
    i18n.print(Msg::AddHintDefault(name.to_string()));
    i18n.print(Msg::AddHintLink(name.to_string()));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn login_command_sets_config_dir() {
        let dir = Path::new("/tmp/some-account");
        let cmd = build_login_command(dir);
        let set = cmd
            .get_envs()
            .find(|(k, _)| *k == std::ffi::OsStr::new("CLAUDE_CONFIG_DIR"));

        assert_eq!(
            set,
            Some((
                std::ffi::OsStr::new("CLAUDE_CONFIG_DIR"),
                Some(std::ffi::OsStr::new("/tmp/some-account"))
            ))
        );
    }

    #[test]
    fn login_command_strips_auth_env_vars() {
        let dir = Path::new("/tmp/some-account");
        let cmd = build_login_command(dir);
        for var in crate::environment::CLAUDE_AUTH_ENV_VARS {
            let removed = cmd
                .get_envs()
                .find(|(k, _)| *k == std::ffi::OsStr::new(*var));
            assert_eq!(
                removed,
                Some((std::ffi::OsStr::new(*var), None)),
                "{var} not stripped"
            );
        }
    }

    #[test]
    fn login_command_uses_claude_auth_login_args() {
        let dir = Path::new("/tmp/some-account");
        let cmd = build_login_command(dir);
        let args: Vec<_> = cmd.get_args().collect();
        assert_eq!(args, vec!["auth", "login"]);
    }
}
