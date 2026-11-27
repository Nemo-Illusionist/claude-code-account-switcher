use std::path::PathBuf;

use crate::config::{AppConfig, is_reserved_name, validate_name};
use crate::i18n::{I18n, Msg};
use crate::seed;

pub fn run(config: &AppConfig, i18n: &I18n, name: &str, from: Option<&str>) {
    if is_reserved_name(name) {
        i18n.print(Msg::ReservedName(name.to_string()));
        std::process::exit(1);
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

    let source = match source_dir(config, name, from) {
        Ok(dir) => dir,
        Err(SourceError::SameAccount) => {
            i18n.print(Msg::CloneSettingsSameAccount(name.to_string()));
            std::process::exit(1);
        }
        Err(SourceError::NotAnAccount(bad)) => {
            i18n.print(Msg::LoginNotFound(bad));
            std::process::exit(1);
        }
        Err(SourceError::InvalidName) => {
            i18n.print(Msg::NameInvalid);
            std::process::exit(1);
        }
        Err(SourceError::NoHome) => {
            eprintln!("clone-settings: cannot determine home directory");
            std::process::exit(1);
        }
    };

    match seed::copy_config(&source, &acc_dir) {
        Ok(report) if report.is_empty() => {
            i18n.print(Msg::SeedNothingToCopy);
        }
        Ok(report) => {
            for entry in &report.copied {
                i18n.print(Msg::SeedCopied(entry.clone()));
            }
        }
        Err(e) => {
            eprintln!("clone-settings: {}", e);
            std::process::exit(1);
        }
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
        return standard(config);
    };
    if is_reserved_name(from) {
        return standard(config);
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
fn standard(_config: &AppConfig) -> Result<PathBuf, SourceError> {
    dirs::home_dir()
        .map(|h| h.join(".claude"))
        .ok_or(SourceError::NoHome)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_config(tag: &str) -> AppConfig {
        let base = std::env::temp_dir().join(format!("cc-clone-{}-{}", tag, std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let config = AppConfig { base_dir: base };
        config.init().unwrap();
        config
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
