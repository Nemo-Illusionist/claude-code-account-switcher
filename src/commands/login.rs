use crate::config::{AppConfig, validate_name};
use crate::environment::strip_claude_auth_env;
use crate::i18n::{I18n, Msg};
use crate::identity;
use std::process::Command;

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
    let mut login_cmd = Command::new("claude");
    login_cmd
        .args(["auth", "login"])
        .env("CLAUDE_CONFIG_DIR", &acc_dir);
    // A leaked ANTHROPIC_API_KEY etc. can make `claude auth login` skip the
    // OAuth flow entirely, or auth a different identity than acc_dir intends.
    strip_claude_auth_env(&mut login_cmd);
    login_cmd.status().expect("Failed to run claude auth login");
    identity::restore_side_effect_keychain(keychain_snapshot);

    i18n.print(Msg::LoginDone);
}

/// Re-login the standard `~/.claude` account. No keychain snapshot/restore
/// here — unlike logging into a *different* account, this login is meant to
/// change the standard account's own credentials.
fn login_default(i18n: &I18n) {
    i18n.print(Msg::LoginStart("default".to_string()));
    let mut login_cmd = Command::new("claude");
    login_cmd.args(["auth", "login"]);
    // Clear any CLAUDE_CONFIG_DIR inherited from a directory link, and tell
    // claude-acc's IDE wrapper (see src/ide.rs / commands/run.rs) not to
    // re-derive one from $PWD — same reasoning as `run default`.
    login_cmd.env_remove("CLAUDE_CONFIG_DIR");
    login_cmd.env("CLAUDE_ACC_RUN_DEFAULT", "1");
    strip_claude_auth_env(&mut login_cmd);
    login_cmd.status().expect("Failed to run claude auth login");

    i18n.print(Msg::LoginDone);
}
