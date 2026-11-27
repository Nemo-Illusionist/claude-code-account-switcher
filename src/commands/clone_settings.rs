use std::path::{Path, PathBuf};

use crate::config::{AppConfig, is_reserved_name, validate_name};
use crate::i18n::{I18n, Msg};
use crate::seed;

pub fn run(config: &AppConfig, i18n: &I18n, name: &str, from: Option<&str>) -> i32 {
    if is_reserved_name(name) {
        i18n.print(Msg::ReservedName(name.to_string()));
        return 1;
    }

    if !validate_name(name) {
        i18n.print(Msg::NameInvalid);
        return 1;
    }
    let acc_dir = config.account_path(name);
    if !acc_dir.is_dir() {
        i18n.print(Msg::LoginNotFound(name.to_string()));
        return 1;
    }

    let source = match source_dir(config, name, from) {
        Ok(dir) => dir,
        Err(SourceError::SameAccount) => {
            i18n.print(Msg::CloneSettingsSameAccount(name.to_string()));
            return 1;
        }
        Err(SourceError::NotAnAccount(bad)) => {
            i18n.print(Msg::LoginNotFound(bad));
            return 1;
        }
        Err(SourceError::InvalidName) => {
            i18n.print(Msg::NameInvalid);
            return 1;
        }
        Err(SourceError::NoHome) => {
            eprintln!("clone-settings: cannot determine home directory");
            return 1;
        }
    };

    match seed::copy_config(&source, &acc_dir) {
        Ok(report) if report.is_empty() => {
            // Named, because it is no longer always `~/.claude/`: saying
            // that about an account the user chose would be a lie.
            i18n.print(Msg::SeedNothingToCopy(home_relative(&source)));
        }
        Ok(report) => {
            for entry in &report.copied {
                i18n.print(Msg::SeedCopied(entry.clone()));
            }
        }
        Err(e) => {
            eprintln!("clone-settings: {}", e);
            return 1;
        }
    }
    0
}

/// A path with the home directory written as `~`, for a message a person
/// reads: `~/.claude` rather than `/Users/someone/.claude`.
fn home_relative(path: &Path) -> String {
    home_relative_to(path, dirs::home_dir().as_deref())
}

/// The decision, with the home directory passed in, so the test does not
/// depend on whose machine it runs on.
fn home_relative_to(path: &Path, home: Option<&Path>) -> String {
    match home.and_then(|h| path.strip_prefix(h).ok().map(PathBuf::from)) {
        Some(rel) if rel.as_os_str().is_empty() => "~".to_string(),
        Some(rel) => format!("~/{}", rel.display()),
        None => path.display().to_string(),
    }
}

#[derive(Debug, PartialEq)]
pub enum SourceError {
    /// `--from` names the account being seeded.
    SameAccount,
    /// `--from` names something that is not an account directory here.
    NotAnAccount(String),
    InvalidName,
    NoHome,
}

/// The directory to copy from.
///
/// `--from` is absent for the original behaviour — the standard `~/.claude`
/// — `default` says the same thing explicitly, and any other name is a
/// managed account. The checks are the ones the rest of the tool applies to
/// an account name, plus one of its own: copying an account onto itself is
/// not a no-op worth running, it is a typo.
fn source_dir(
    config: &AppConfig,
    target: &str,
    from: Option<&str>,
) -> Result<PathBuf, SourceError> {
    let Some(from) = from else {
        return standard();
    };
    if is_reserved_name(from) {
        return standard();
    }
    if !validate_name(from) {
        return Err(SourceError::InvalidName);
    }
    if from == target {
        return Err(SourceError::SameAccount);
    }
    let dir = config.account_path(from);
    if !dir.is_dir() {
        return Err(SourceError::NotAnAccount(from.to_string()));
    }
    Ok(dir)
}

/// The standard account's config dir. It is `~/.claude`, not something under
/// the switch dir, which is why it cannot be `account_path("default")`.
fn standard() -> Result<PathBuf, SourceError> {
    dirs::home_dir()
        .map(|h| h.join(".claude"))
        .ok_or(SourceError::NoHome)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::Lang;

    fn temp_config(tag: &str) -> AppConfig {
        let base = std::env::temp_dir().join(format!("cc-clone-{}-{}", tag, std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let config = AppConfig { base_dir: base };
        config.init().unwrap();
        config
    }

    #[test]
    fn the_command_copies_from_the_account_from_names() {
        // The wiring, not just the decision: `run` used to call
        // `copy_user_config`, which could only ever read `~/.claude/`, and
        // a test of `source_dir` alone would not have noticed it staying
        // that way.
        let c = temp_config("wiring");
        let work = c.accounts_dir().join("work");
        let target = c.accounts_dir().join("work-2");
        std::fs::create_dir_all(&work).unwrap();
        std::fs::create_dir_all(&target).unwrap();
        std::fs::write(work.join("CLAUDE.md"), "from work").unwrap();
        let i18n = I18n { lang: Lang::En };

        assert_eq!(run(&c, &i18n, "work-2", Some("work")), 0);
        assert_eq!(
            std::fs::read_to_string(target.join("CLAUDE.md")).unwrap(),
            "from work"
        );

        // And the refusals come back as exit codes rather than as a process
        // that has already gone.
        assert_eq!(run(&c, &i18n, "work", Some("work")), 1);
        assert_eq!(run(&c, &i18n, "work-2", Some("nope")), 1);
        assert_eq!(run(&c, &i18n, "work-2", Some("work.lock")), 1);
        assert_eq!(run(&c, &i18n, "nope", Some("work")), 1);
        assert_eq!(run(&c, &i18n, "default", None), 1);

        // Nothing left to copy is still a success, not a failure.
        let bare = c.accounts_dir().join("bare");
        let bare2 = c.accounts_dir().join("bare-2");
        std::fs::create_dir_all(&bare).unwrap();
        std::fs::create_dir_all(&bare2).unwrap();
        assert_eq!(run(&c, &i18n, "bare-2", Some("bare")), 0);

        let _ = std::fs::remove_dir_all(&c.base_dir);
    }

    #[test]
    fn a_source_under_home_is_named_with_a_tilde() {
        // The "nothing to copy" message names the source now, because it is
        // no longer always `~/.claude/`. It must not spell out the home
        // directory of whoever ran it. Expected values by hand.
        let home = Path::new("/Users/someone");
        assert_eq!(
            home_relative_to(&home.join(".claude"), Some(home)),
            "~/.claude"
        );
        assert_eq!(
            home_relative_to(&home.join(".claude-switch/accounts/work"), Some(home)),
            "~/.claude-switch/accounts/work"
        );
        // Outside home, and with no home at all: printed as it is, never
        // with a `~` that would point somewhere else.
        assert_eq!(
            home_relative_to(Path::new("/srv/accounts/work"), Some(home)),
            "/srv/accounts/work"
        );
        assert_eq!(
            home_relative_to(Path::new("/Users/someone-else/x"), Some(home)),
            "/Users/someone-else/x"
        );
        assert_eq!(home_relative_to(home, Some(home)), "~");
    }

    #[test]
    fn the_source_is_the_account_from_names() {
        let c = temp_config("source");
        let work = c.accounts_dir().join("work");
        std::fs::create_dir_all(&work).unwrap();

        assert_eq!(source_dir(&c, "work-2", Some("work")), Ok(work));
        // `default` is the standard account, which lives outside the switch
        // dir — never `accounts/default`.
        let home = dirs::home_dir().unwrap().join(".claude");
        assert_eq!(source_dir(&c, "work-2", Some("default")), Ok(home.clone()));
        // Absent is what the command did before `--from` existed.
        assert_eq!(source_dir(&c, "work-2", None), Ok(home));

        // An account that is not there, a name no command would take, and
        // the account being seeded.
        assert_eq!(
            source_dir(&c, "work-2", Some("nope")),
            Err(SourceError::NotAnAccount("nope".to_string()))
        );
        assert_eq!(
            source_dir(&c, "work-2", Some("work.lock")),
            Err(SourceError::InvalidName)
        );
        assert_eq!(
            source_dir(&c, "work", Some("work")),
            Err(SourceError::SameAccount)
        );

        let _ = std::fs::remove_dir_all(&c.base_dir);
    }
}
