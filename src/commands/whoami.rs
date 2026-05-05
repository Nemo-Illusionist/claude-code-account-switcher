// `claude-acc whoami` — print the most-identifying string for the active
// account, suitable for use in shell prompts and conditional scripts.
//
// Resolution order (matches `status` for active-account detection):
//   1. Cached email (managed account or standard ~/.claude/)
//   2. Account name (managed but no cached email)
//   3. The literal `default` (standard with no cached identity)
//
// Always exits 0; output is always non-empty so scripts can compare safely.

use crate::config::AppConfig;
use crate::identity;
use crate::resolve;

pub fn run(config: &AppConfig) {
    let cwd = std::env::current_dir().expect("Cannot get current directory");

    // 1. Linked directory wins.
    if let Some(ld) = resolve::find_linked_dir(config, &cwd)
        && let Ok(Some(acc)) = config.get_link(&ld)
    {
        if acc == "default" {
            println!("{}", standard_label(config));
        } else {
            println!("{}", account_label(config, &acc));
        }
        return;
    }

    // 2. Configured default.
    if let Ok(Some(acc)) = config.get_default() {
        println!("{}", account_label(config, &acc));
        return;
    }

    // 3. Standard ~/.claude/.
    println!("{}", standard_label(config));
}

fn account_label(config: &AppConfig, acc: &str) -> String {
    let acc_dir = config.account_path(acc);
    identity::read_cache(&acc_dir)
        .and_then(|c| c.email)
        .unwrap_or_else(|| acc.to_string())
}

fn standard_label(config: &AppConfig) -> String {
    let cache_path = identity::default_cache_path(&config.base_dir);
    identity::read_cache_at(&cache_path)
        .and_then(|c| c.email)
        .unwrap_or_else(|| "default".to_string())
}
