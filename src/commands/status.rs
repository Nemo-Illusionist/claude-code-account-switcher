use crate::config::AppConfig;
use crate::i18n::{I18n, Msg};
use crate::resolve;

pub fn run(config: &AppConfig, i18n: &I18n) {
    let cwd = std::env::current_dir().expect("Cannot get current directory");

    let linked_dir = resolve::find_linked_dir(config, &cwd);

    if let Some(ref ld) = linked_dir {
        let account = config.get_link(ld).ok().flatten();
        if let Some(ref acc) = account {
            let dir_name = std::path::Path::new(ld)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(ld);
            let info = i18n.msg(Msg::StatusLinked(dir_name.to_string()));
            i18n.print(Msg::StatusActive(acc.clone(), info));
            return;
        }
    }

    if let Ok(Some(ref acc)) = config.get_default() {
        let info = i18n.msg(Msg::StatusDefault);
        i18n.print(Msg::StatusActive(acc.clone(), info));
        return;
    }

    i18n.print(Msg::StatusStandard);
}
