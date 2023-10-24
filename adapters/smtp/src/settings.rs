use crate::models::SmtpConfig;

use lazy_static::lazy_static;
use std::{path::PathBuf, sync::Mutex};

lazy_static! {
    #[derive(Debug)]
    pub(crate) static ref SMTP_CONFIG: Mutex<Option<SmtpConfig>> = Mutex::new(None);
}

pub async fn init_config_from_file(config_path: PathBuf) {
    SMTP_CONFIG.lock().unwrap().replace(
        match SmtpConfig::from_default_config_file(config_path) {
            Ok(config) => config,
            Err(err) => {
                panic!("Error detected on initialize smtp config: {err}")
            }
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_init_config_from_file() {
        init_config_from_file(PathBuf::from("config.toml")).await;
        let config = SMTP_CONFIG.lock().unwrap();
        assert!(config.is_some());
    }
}
