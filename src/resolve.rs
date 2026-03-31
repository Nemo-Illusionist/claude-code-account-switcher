use std::path::Path;
use crate::config::AppConfig;

/// Walk up from `dir` to root, checking links for each ancestor.
pub fn resolve_account(config: &AppConfig, dir: &Path) -> Option<String> {
    let mut current = dir.to_path_buf();
    loop {
        if let Some(dir_str) = current.to_str() {
            if let Ok(Some(account)) = config.get_link(dir_str) {
                return Some(account);
            }
        }
        if !current.pop() {
            break;
        }
    }
    // Fallback to default
    config.get_default().ok().flatten()
}

/// Find the linked directory (for status display).
pub fn find_linked_dir(config: &AppConfig, dir: &Path) -> Option<String> {
    let mut current = dir.to_path_buf();
    loop {
        if let Some(dir_str) = current.to_str() {
            if let Ok(Some(_)) = config.get_link(dir_str) {
                return Some(dir_str.to_string());
            }
        }
        if !current.pop() {
            break;
        }
    }
    None
}
