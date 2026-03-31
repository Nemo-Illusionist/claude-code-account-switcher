use std::fs;
use std::io;
use std::path::PathBuf;

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
        if old_repos.exists() && fs::read_to_string(self.links_path()).map(|s| s.is_empty()).unwrap_or(true) {
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
            if entry.file_type()?.is_dir() {
                if let Some(name) = entry.file_name().to_str() {
                    accounts.push(name.to_string());
                }
            }
        }
        accounts.sort();
        Ok(accounts)
    }

    // --- config file ---

    pub fn get_default(&self) -> io::Result<Option<String>> {
        let content = fs::read_to_string(self.config_path())?;
        for line in content.lines() {
            if let Some(val) = line.strip_prefix("default=") {
                let val = val.trim();
                if !val.is_empty() {
                    return Ok(Some(val.to_string()));
                }
            }
        }
        Ok(None)
    }

    pub fn set_default(&self, name: &str) -> io::Result<()> {
        fs::write(self.config_path(), format!("default={}\n", name))
    }

    pub fn clear_default(&self) -> io::Result<()> {
        fs::write(self.config_path(), "default=\n")
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
        let content = if content.is_empty() { String::new() } else { content + "\n" };
        fs::write(self.links_path(), content)
    }
}
