use crate::models::SmtpConfig;

use lazy_static::lazy_static;
use myc_config::optional_config::OptionalConfig;
use std::{path::PathBuf, sync::Mutex};

lazy_static! {
    #[derive(Debug)]
    pub(crate) static ref SMTP_CONFIG: Mutex<Option<OptionalConfig<SmtpConfig>>> = Mutex::new(None);
}

pub async fn init_smtp_config_from_file(
    config_path: Option<PathBuf>,
    config_instance: Option<OptionalConfig<SmtpConfig>>,
) {
    if let Some(config) = config_instance {
        SMTP_CONFIG.lock().unwrap().replace(config);
        return;
    }

    if config_path.is_none() {
        panic!("Error detected on initialize smtp config: config path is none");
    }

    SMTP_CONFIG.lock().unwrap().replace(
        match SmtpConfig::from_default_config_file(config_path.unwrap()) {
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
        init_smtp_config_from_file(Some(PathBuf::from("config.toml")), None)
            .await;
        let config = SMTP_CONFIG.lock().unwrap();
        assert!(config.is_some());
    }
}
