pub mod activate;
pub mod add;
pub mod clone_settings;
pub mod completions;
pub mod default;
pub mod doctor;
pub mod import;
pub mod init;
pub mod install;
pub mod link;
pub mod links;
pub mod list;
pub mod login;
pub mod remove;
pub mod reset;
pub mod run;
pub mod status;
pub mod statusline;
pub mod unlink;
pub mod update;
pub mod usage;
pub mod whoami;

use crate::config::AppConfig;
use crate::identity;
use std::path::PathBuf;

/// `(label, cache_path)` pairs for every already-known account except the
/// one at `exclude_label` — every managed account plus the standard
/// `~/.claude` account (labeled `"~/.claude/"`). Feeds
/// `identity::find_duplicate_account`'s `known` argument, used by `add` and
/// `login` to warn when a freshly-authenticated account turns out to share
/// an identity with one that already exists.
fn known_account_cache_paths(config: &AppConfig, exclude_label: &str) -> Vec<(String, PathBuf)> {
    let mut known: Vec<(String, PathBuf)> = config
        .list_accounts()
        .unwrap_or_default()
        .into_iter()
        .filter(|acc| acc != exclude_label)
        .map(|acc| {
            let cache_path = config.account_path(&acc).join(".account-info.json");
            (acc, cache_path)
        })
        .collect();
    if exclude_label != "~/.claude/" {
        known.push((
            "~/.claude/".to_string(),
            identity::default_cache_path(&config.base_dir),
        ));
    }
    known
}
