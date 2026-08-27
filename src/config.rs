use std::fs;
use std::io;
use std::path::PathBuf;

/// Account names: ASCII letters, digits, hyphens, underscores. Rejects path
/// separators, regex metacharacters, whitespace, and unicode — anything
/// unsafe in a filesystem path or shell expansion.
pub fn validate_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Whether `name` is the reserved "default" name — not a real managed
/// account directory, used across commands to mean "the standard ~/.claude
/// account". `add`, `remove`, and `clone-settings` reject it as a target
/// (there's nothing to add/remove/clone-into for the standard account).
pub fn is_reserved_name(name: &str) -> bool {
    name == "default"
}

pub struct AppConfig {
    pub base_dir: PathBuf,
}

impl AppConfig {
    pub fn new() -> Self {
        let home = dirs::home_dir().expect("Cannot determine home directory");
        Self {
            base_dir: home.join(".claude-switch"),
        }
    }

    pub fn accounts_dir(&self) -> PathBuf {
        self.base_dir.join("accounts")
    }

    pub fn config_path(&self) -> PathBuf {
        self.base_dir.join("config")
    }

    pub fn links_path(&self) -> PathBuf {
        self.base_dir.join("links")
    }

    pub fn init(&self) -> io::Result<()> {
        fs::create_dir_all(self.accounts_dir())?;
        if !self.config_path().exists() {
            fs::write(self.config_path(), "default=\n")?;
        }
        if !self.links_path().exists() {
            fs::write(self.links_path(), "")?;
        }
        // Migration: repos → links
        let old_repos = self.base_dir.join("repos");
        if old_repos.exists()
            && fs::read_to_string(self.links_path())
                .map(|s| s.is_empty())
                .unwrap_or(true)
        {
            fs::rename(&old_repos, self.links_path())?;
        }
        Ok(())
    }

    pub fn account_path(&self, name: &str) -> PathBuf {
        self.accounts_dir().join(name)
    }

    pub fn account_exists(&self, name: &str) -> bool {
        self.account_path(name).is_dir()
    }

    pub fn list_accounts(&self) -> io::Result<Vec<String>> {
        let dir = self.accounts_dir();
        if !dir.exists() {
            return Ok(vec![]);
        }
        let mut accounts = Vec::new();
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir()
                && let Some(name) = entry.file_name().to_str()
            {
                accounts.push(name.to_string());
            }
        }
        accounts.sort();
        Ok(accounts)
    }

    // --- config file ---

    /// The `key=value` lines of the config file, in file order. Unparseable
    /// lines are dropped rather than preserved — the file is ours, and a
    /// stray line is more likely damage than something worth keeping.
    fn read_settings(&self) -> Vec<(String, String)> {
        let Ok(content) = fs::read_to_string(self.config_path()) else {
            return Vec::new();
        };
        content
            .lines()
            .filter_map(|line| line.split_once('='))
            .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
            .filter(|(k, _)| !k.is_empty())
            .collect()
    }

    fn write_settings(&self, settings: &[(String, String)]) -> io::Result<()> {
        let content: String = settings
            .iter()
            .map(|(k, v)| format!("{}={}\n", k, v))
            .collect();
        fs::write(self.config_path(), content)
    }

    /// The value of `key`, or `None` when it is absent or empty. Empty is
    /// treated as absent so `default=` keeps meaning "no default set".
    pub fn get_setting(&self, key: &str) -> Option<String> {
        self.read_settings()
            .into_iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v)
            .filter(|v| !v.is_empty())
    }

    /// Set `key`, leaving every other setting in the file untouched. New keys
    /// are appended; existing ones are updated in place.
    pub fn set_setting(&self, key: &str, value: &str) -> io::Result<()> {
        let mut settings = self.read_settings();
        match settings.iter_mut().find(|(k, _)| k == key) {
            Some(entry) => entry.1 = value.to_string(),
            None => settings.push((key.to_string(), value.to_string())),
        }
        self.write_settings(&settings)
    }

    pub fn get_default(&self) -> io::Result<Option<String>> {
        Ok(self.get_setting("default"))
    }

    pub fn set_default(&self, name: &str) -> io::Result<()> {
        self.set_setting("default", name)
    }

    pub fn clear_default(&self) -> io::Result<()> {
        self.set_setting("default", "")
    }

    // --- links file ---

    pub fn all_links(&self) -> io::Result<Vec<(String, String)>> {
        let content = fs::read_to_string(self.links_path())?;
        let mut links = Vec::new();
        for line in content.lines() {
            if let Some((dir, account)) = line.split_once('=') {
                let dir = dir.trim();
                let account = account.trim();
                if !dir.is_empty() && !account.is_empty() {
                    links.push((dir.to_string(), account.to_string()));
                }
            }
        }
        Ok(links)
    }

    pub fn get_link(&self, dir: &str) -> io::Result<Option<String>> {
        let links = self.all_links()?;
        for (d, acc) in &links {
            if d == dir {
                return Ok(Some(acc.clone()));
            }
        }
        Ok(None)
    }

    pub fn set_link(&self, dir: &str, account: &str) -> io::Result<()> {
        let mut links = self.all_links()?;
        links.retain(|(d, _)| d != dir);
        links.push((dir.to_string(), account.to_string()));
        self.write_links(&links)
    }

    pub fn remove_link(&self, dir: &str) -> io::Result<bool> {
        let mut links = self.all_links()?;
        let before = links.len();
        links.retain(|(d, _)| d != dir);
        if links.len() == before {
            return Ok(false);
        }
        self.write_links(&links)?;
        Ok(true)
    }

    pub fn remove_links_for_account(&self, account: &str) -> io::Result<()> {
        let mut links = self.all_links()?;
        links.retain(|(_, a)| a != account);
        self.write_links(&links)
    }

    fn write_links(&self, links: &[(String, String)]) -> io::Result<()> {
        let content: String = links
            .iter()
            .map(|(d, a)| format!("{}={}", d, a))
            .collect::<Vec<_>>()
            .join("\n");
        let content = if content.is_empty() {
            String::new()
        } else {
            content + "\n"
        };
        fs::write(self.links_path(), content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_name_accepts_alphanumeric() {
        assert!(validate_name("work"));
        assert!(validate_name("personal-2"));
        assert!(validate_name("a_b"));
        assert!(validate_name("WORK"));
        assert!(validate_name("123"));
        assert!(validate_name("a"));
    }

    #[test]
    fn validate_name_rejects_empty() {
        assert!(!validate_name(""));
    }

    #[test]
    fn validate_name_rejects_path_separators_and_traversal() {
        assert!(!validate_name("a/b"));
        assert!(!validate_name("../etc"));
        assert!(!validate_name(".."));
        assert!(!validate_name("."));
    }

    #[test]
    fn validate_name_rejects_whitespace() {
        assert!(!validate_name("a b"));
        assert!(!validate_name("a\tb"));
        assert!(!validate_name(" leading"));
    }

    #[test]
    fn validate_name_rejects_regex_metachars() {
        assert!(!validate_name("a.b"));
        assert!(!validate_name("a[b"));
        assert!(!validate_name("a+b"));
        assert!(!validate_name("a*b"));
    }

    #[test]
    fn validate_name_rejects_links_format_break() {
        // `=` would corrupt the path=name links file format.
        assert!(!validate_name("a=b"));
    }

    #[test]
    fn validate_name_rejects_unicode() {
        assert!(!validate_name("работа"));
        assert!(!validate_name("café"));
    }

    fn temp_config(tag: &str) -> AppConfig {
        let base = std::env::temp_dir().join(format!("cc-config-{}-{}", tag, std::process::id()));
        let _ = fs::remove_dir_all(&base);
        let config = AppConfig { base_dir: base };
        config.init().unwrap();
        config
    }

    #[test]
    fn setting_round_trips() {
        let c = temp_config("round-trip");
        assert_eq!(c.get_setting("resume_hook"), None);
        c.set_setting("resume_hook", "off").unwrap();
        assert_eq!(c.get_setting("resume_hook").as_deref(), Some("off"));
        let _ = fs::remove_dir_all(&c.base_dir);
    }

    #[test]
    fn setting_an_empty_value_reads_back_as_absent() {
        // `default=` is how "no default account" is stored, so an empty
        // value has to be indistinguishable from a missing key.
        let c = temp_config("empty");
        c.set_setting("default", "").unwrap();
        assert_eq!(c.get_setting("default"), None);
        let _ = fs::remove_dir_all(&c.base_dir);
    }

    #[test]
    fn setting_one_key_leaves_the_others_alone() {
        // Regression: set_default used to rewrite the whole file, which would
        // silently drop every other setting.
        let c = temp_config("preserve");
        c.set_setting("resume_hook", "off").unwrap();
        c.set_default("work").unwrap();
        assert_eq!(c.get_setting("resume_hook").as_deref(), Some("off"));
        assert_eq!(c.get_default().unwrap().as_deref(), Some("work"));

        c.clear_default().unwrap();
        assert_eq!(c.get_setting("resume_hook").as_deref(), Some("off"));
        assert_eq!(c.get_default().unwrap(), None);
        let _ = fs::remove_dir_all(&c.base_dir);
    }

    #[test]
    fn updating_a_key_does_not_append_a_duplicate() {
        let c = temp_config("no-dup");
        c.set_setting("resume_hook", "off").unwrap();
        c.set_setting("resume_hook", "on").unwrap();
        let raw = fs::read_to_string(c.config_path()).unwrap();
        assert_eq!(raw.matches("resume_hook=").count(), 1, "{}", raw);
        assert_eq!(c.get_setting("resume_hook").as_deref(), Some("on"));
        let _ = fs::remove_dir_all(&c.base_dir);
    }

    #[test]
    fn is_reserved_name_matches_only_default() {
        assert!(is_reserved_name("default"));
        assert!(!is_reserved_name("work"));
        assert!(!is_reserved_name("Default"));
        assert!(!is_reserved_name(""));
    }
}
