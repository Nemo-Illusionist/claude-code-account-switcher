use crate::config::{AppConfig, validate_name};
use crate::environment::strip_claude_auth_env;
use crate::i18n::{I18n, Msg};
use crate::identity;
use std::process::Command;

pub fn run(config: &AppConfig, i18n: &I18n, name: &str) {
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
