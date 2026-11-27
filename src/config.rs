use std::fs;
use std::io;
use std::path::{Path, PathBuf};

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

/// The contents of `accounts/`, as `read_accounts_dir` reads them.
///
/// The two reported lists are split by what the user can do about them, not
/// by what the entry is, because that is what the hint has to say — and a
/// hint that is false for part of its own list is worse than none.
///
/// The name decides first. Every command takes an account by name and
/// checks the name before anything else, so an entry named something
/// `validate_name` rejects is untouchable whatever it is: `run`, `remove`
/// and `login` all refuse it, symlink or not. Only among names a command
/// would accept does the kind of entry matter.
#[derive(Debug, Default, PartialEq)]
pub struct AccountsDir {
    /// Directories whose name a command here could have created.
    pub accounts: Vec<String>,
    /// Entries no command can act on: a name `validate_name` rejects, or an
    /// acceptable name on something that is not a directory. `remove` takes
    /// only an account directory, so it cannot clear either kind up.
    pub unnamed: Vec<String>,
    /// Symlinks whose name a command would accept, so `run` and `remove`
    /// reach them — `is_dir` follows a symlink — while `list` skips them and
    /// `gui::account_dir` refuses storage behind one outright.
    pub linked: Vec<String>,
}

// Required, not gratuitous: since `config` became a library module, `new()`
// is reachable as library API, so `clippy::new_without_default` fires and
// `-D warnings` fails the build without this impl. Note it inherits `new()`'s
// `expect("Cannot determine home directory")`: unlike most `Default` impls it
// can panic. That is not new behaviour, since every caller of `new()` already
// panics there.
impl Default for AppConfig {
    fn default() -> Self {
        Self::new()
    }
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

    /// Every managed account, by name.
    pub fn list_accounts(&self) -> io::Result<Vec<String>> {
        Ok(self.read_accounts_dir()?.accounts)
    }

    /// What is under `accounts/`, split into the accounts and everything
    /// else.
    ///
    /// No command here can create an entry whose name `validate_name`
    /// rejects — `add`, `login`, `import` and `clone-settings` all refuse one
    /// before touching the filesystem. Enumerating them anyway turned a
    /// stray `accounts/work.lock` into a row of `list`, a line of `usage`, a
    /// completion candidate and a `doctor` finding that advised
    /// `claude-acc login work.lock`, which rejects its own argument — and
    /// into an account the `ide` pass then wrote a symlink into.
    ///
    /// The rest is reported rather than dropped, so a filtered-out entry
    /// stays discoverable instead of becoming invisible — see `AccountsDir`
    /// for why the two reported lists are separate.
    ///
    /// A symlink is in neither account list: `file_type()` does not follow
    /// one, which is why `list_accounts` never showed one.
    ///
    /// Dotfiles are skipped entirely, not reported. `.DS_Store` appears the
    /// moment anyone opens the directory in Finder, and a hint naming it on
    /// every run — about a file that comes straight back — is worse than
    /// silence. It also keeps this in step with `claude-switch.sh`, whose
    /// globs do not match dotfiles without `GLOB_DOTS`.
    pub fn read_accounts_dir(&self) -> io::Result<AccountsDir> {
        let dir = self.accounts_dir();
        let mut out = AccountsDir::default();
        if !dir.exists() {
            return Ok(out);
        }
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            // Lossy, not skipped: a name that is not UTF-8 is exactly the
            // sort of entry that should not go unmentioned.
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                continue;
            }
            // The name first: a command that refuses the name never gets
            // as far as looking at the entry, so what the entry is cannot
            // change the answer.
            if !validate_name(&name) {
                out.unnamed.push(name);
                continue;
            }
            let kind = entry.file_type()?;
            if kind.is_symlink() {
                out.linked.push(name);
            } else if kind.is_dir() {
                out.accounts.push(name);
            } else {
                out.unnamed.push(name);
            }
        }
        out.linked.sort();
        out.unnamed.sort();
        out.accounts.sort();
        Ok(out)
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
        replace_file(&self.config_path(), &content)
    }

    /// Run one read-modify-write of `config` or `links` under an exclusive
    /// lock on `.config.lock`, so two processes saving at the same moment
    /// cannot each drop the other's change. Advisory:
    /// `claude-switch.sh` takes the same file with `zsystem flock`, whose
    /// fcntl lock excludes this one on macOS.
    ///
    /// This blocks with no timeout, deliberately and unlike the shell side.
    /// `claude-switch.sh` waits five seconds and then edits unlocked, so a
    /// stuck holder cannot wedge someone's interactive shell. Here the caller
    /// is a one-shot command, so waiting is the safer failure: proceeding
    /// unlocked is how a binding gets lost, which is the bug this exists to
    /// prevent. A holder keeps the lock only for one small file rewrite, and
    /// the OS releases it when that process exits however it exits.
    fn locked<T>(&self, edit: impl FnOnce() -> io::Result<T>) -> io::Result<T> {
        let lock = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(self.base_dir.join(".config.lock"))?;
        lock.lock()?;
        edit()
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
        self.locked(|| {
            let mut settings = self.read_settings();
            match settings.iter_mut().find(|(k, _)| k == key) {
                Some(entry) => entry.1 = value.to_string(),
                None => settings.push((key.to_string(), value.to_string())),
            }
            self.write_settings(&settings)
        })
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
        self.locked(|| {
            let mut links = self.all_links()?;
            links.retain(|(d, _)| d != dir);
            links.push((dir.to_string(), account.to_string()));
            self.write_links(&links)
        })
    }

    pub fn remove_link(&self, dir: &str) -> io::Result<bool> {
        self.locked(|| {
            let mut links = self.all_links()?;
            let before = links.len();
            links.retain(|(d, _)| d != dir);
            if links.len() == before {
                return Ok(false);
            }
            self.write_links(&links)?;
            Ok(true)
        })
    }

    pub fn remove_links_for_account(&self, account: &str) -> io::Result<()> {
        self.locked(|| {
            let mut links = self.all_links()?;
            links.retain(|(_, a)| a != account);
            self.write_links(&links)
        })
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
        replace_file(&self.links_path(), &content)
    }
}

/// Sibling temp + rename, as `identity::write_lock_at` does. A plain write
/// truncates first, so a reader — or a crash — mid-write sees an empty file,
/// and for `links` that is every binding gone. Callers hold `locked`, so the
/// fixed temp name cannot collide.
/// Where `replace_file` stages its write for `path`.
///
/// `pid` is passed in rather than read here so the naming rule is assertable
/// without spawning a process.
fn tmp_path(path: &Path, pid: u32) -> PathBuf {
    path.with_extension(format!("tmp.{}", pid))
}

fn replace_file(path: &Path, content: &str) -> io::Result<()> {
    // The temporary name carries the process id, so two writers cannot share
    // it. Under the lock they are already serialised — but `locked` is
    // advisory and `claude-switch.sh` deliberately falls back to an unlocked
    // edit when `zsh/system` is missing or the lock does not come free, which
    // is exactly the state where a single fixed `config.tmp` would let two
    // processes interleave their bytes and then rename the result into place.
    let tmp = tmp_path(path, std::process::id());
    fs::write(&tmp, content)?;
    fs::rename(&tmp, path).inspect_err(|_| {
        let _ = fs::remove_file(&tmp);
    })
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
    #[cfg(unix)]
    fn a_symlink_is_reported_apart_from_a_name_no_command_accepts() {
        // The two need different words: `remove` and `run` follow a symlink
        // (`is_dir` does), so "nothing of ours can clear it up" is false for
        // one and true for the other, and "delete it yourself" pointed at
        // `rm` on a link into live data.
        let c = temp_config("list-symlink");
        let dir = c.accounts_dir();
        fs::create_dir_all(dir.join("plain")).unwrap();
        fs::create_dir_all(dir.join("work.lock")).unwrap();
        let outside = c.base_dir.join("elsewhere");
        fs::create_dir_all(&outside).unwrap();
        std::os::unix::fs::symlink(&outside, dir.join("linked")).unwrap();
        // A symlink named something no command accepts belongs with the
        // untouchable ones, not with the symlinks: `run` and `remove` stop
        // at the name and never reach it, so "they follow them" would be
        // false and "make it a real directory" would not help — a directory
        // of that name is not an account either.
        std::os::unix::fs::symlink(&outside, dir.join("linked.2")).unwrap();
        // Likewise by kind: an acceptable name on something that is not a
        // directory is not an account, and `remove` refuses it too.
        fs::write(c.base_dir.join("target.txt"), "x").unwrap();
        std::os::unix::fs::symlink(c.base_dir.join("target.txt"), dir.join("tofile")).unwrap();
        std::os::unix::fs::symlink(c.base_dir.join("gone"), dir.join("dangling")).unwrap();
        fs::write(dir.join("personal"), "a file with a name add would take").unwrap();

        let read = c.read_accounts_dir().unwrap();
        assert_eq!(read.accounts, vec!["plain"]);
        assert_eq!(read.unnamed, vec!["linked.2", "personal", "work.lock"]);
        assert_eq!(read.linked, vec!["dangling", "linked", "tofile"]);
        let _ = fs::remove_dir_all(&c.base_dir);
    }

    #[test]
    fn a_dotfile_is_not_reported_at_all() {
        // `.DS_Store` arrives whenever anyone opens the directory in Finder.
        // A line naming it on every run, about a file that comes straight
        // back, is worse than silence — and `claude-switch.sh`'s globs skip
        // dotfiles anyway, so reporting them here would split the two.
        let c = temp_config("list-dotfiles");
        let dir = c.accounts_dir();
        fs::create_dir_all(dir.join("work")).unwrap();
        fs::write(dir.join(".DS_Store"), "finder").unwrap();
        fs::create_dir_all(dir.join(".hidden")).unwrap();

        let read = c.read_accounts_dir().unwrap();
        assert_eq!(read.accounts, vec!["work"]);
        assert_eq!(read.unnamed, Vec::<String>::new());
        assert_eq!(read.linked, Vec::<String>::new());
        let _ = fs::remove_dir_all(&c.base_dir);
    }

    #[test]
    fn a_directory_no_command_could_have_created_is_not_an_account() {
        // Regression: `accounts/work.lock` — a directory left beside an
        // account by something else — was listed as an account, and `doctor`
        // told the user to run `claude-acc login work.lock`, which `login`
        // rejects. Every name here is checked against `validate_name`'s own
        // rules by hand, not by calling it.
        let c = temp_config("list-accounts");
        let dir = c.accounts_dir();
        // No two names here differ only by case: macOS and Windows would
        // fold them onto one directory and the fixture would not be what it
        // reads as.
        for name in ["work", "work-2", "a_b", "work.lock", "work lock", ".hidden"] {
            fs::create_dir_all(dir.join(name)).unwrap();
        }
        fs::write(dir.join("notes.txt"), "not a directory").unwrap();

        assert_eq!(c.list_accounts().unwrap(), vec!["a_b", "work", "work-2"]);
        // What is left over is reported, except the dotfile: see
        // `a_dotfile_is_not_reported_at_all`.
        assert_eq!(
            c.read_accounts_dir().unwrap().unnamed,
            vec!["notes.txt", "work lock", "work.lock"]
        );
        let _ = fs::remove_dir_all(&c.base_dir);
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
    fn concurrent_writers_do_not_lose_each_others_links() {
        // Regression: set_link was an unlocked read-modify-write, so two
        // processes saving at once each wrote back the file they had read
        // and one binding silently vanished. Separate `AppConfig`s open
        // separate lock handles, exactly like two processes.
        let c = temp_config("concurrent");
        let base = c.base_dir.clone();
        std::thread::scope(|s| {
            for t in 0..8 {
                let base = base.clone();
                s.spawn(move || {
                    let c = AppConfig { base_dir: base };
                    for i in 0..25 {
                        c.set_link(&format!("/p/{t}/{i}"), "work").unwrap();
                    }
                });
            }
        });
        assert_eq!(c.all_links().unwrap().len(), 200);
        let _ = fs::remove_dir_all(&c.base_dir);
    }

    #[test]
    fn a_write_leaves_no_temp_file_behind() {
        let c = temp_config("no-tmp");
        c.set_link("/p", "work").unwrap();
        c.set_default("work").unwrap();
        let mut names: Vec<_> = fs::read_dir(&c.base_dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        assert_eq!(names, [".config.lock", "accounts", "config", "links"]);
        let _ = fs::remove_dir_all(&c.base_dir);
    }

    // Follow-up to #160: the staging name was a fixed `config.tmp`, shared by
    // every process. Under the lock that is harmless, but `locked` is
    // advisory and `claude-switch.sh` falls back to an unlocked edit when
    // `zsh/system` is missing or the lock does not come free within five
    // seconds — and in that state two writers would interleave their bytes
    // into one temporary and rename the result into place.
    #[test]
    fn the_staging_name_is_specific_to_one_process() {
        let p = Path::new("/base/links");
        assert_ne!(
            tmp_path(p, 101),
            tmp_path(p, 102),
            "two processes must not stage through the same file"
        );
        assert_eq!(tmp_path(p, 101), Path::new("/base/links.tmp.101"));
    }

    // It must still sit beside the target: `rename` is only atomic within one
    // filesystem, and staging in the system temp directory would cross one.
    #[test]
    fn the_staging_file_is_a_sibling_of_its_target() {
        let p = Path::new("/base/links");
        assert_eq!(tmp_path(p, 7).parent(), p.parent());
    }

    // And it must not collide with a real file. Both names this tool writes
    // are extensionless, so appending rather than replacing is what keeps
    // `config` and `links` distinguishable once staged.
    #[test]
    fn the_staging_names_of_the_two_files_differ() {
        assert_ne!(
            tmp_path(Path::new("/base/config"), 7),
            tmp_path(Path::new("/base/links"), 7)
        );
    }

    #[test]
    fn is_reserved_name_matches_only_default() {
        assert!(is_reserved_name("default"));
        assert!(!is_reserved_name("work"));
        assert!(!is_reserved_name("Default"));
        assert!(!is_reserved_name(""));
    }
}
