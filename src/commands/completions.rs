use crate::config::AppConfig;

pub fn run(config: &AppConfig, what: &str) {
    match what {
        "accounts" => {
            let accounts = config.list_accounts().unwrap_or_default();
            for acc in accounts {
                println!("{}", acc);
            }
        }
        _ => {}
    }
}
