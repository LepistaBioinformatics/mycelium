use crate::{
    models::{QueueConfig, SmtpConfig},
    repositories::init_queue_client_from_url,
};

use lazy_static::lazy_static;
use myc_config::optional_config::OptionalConfig;
use std::{path::PathBuf, sync::Mutex};

lazy_static! {
    #[derive(Debug)]
    pub(crate) static ref SMTP_CONFIG: Mutex<Option<OptionalConfig<SmtpConfig>>> = Mutex::new(None);

    #[derive(Debug)]
    pub(crate) static ref QUEUE_CONFIG: Mutex<Option<QueueConfig>> = Mutex::new(None);
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

pub async fn init_queue_config_from_file(
    config_path: Option<PathBuf>,
    config_instance: Option<OptionalConfig<QueueConfig>>,
) {
    if let Some(config) = config_instance {
        if let OptionalConfig::Enabled(config) = config {
            if let Err(err) =
                init_queue_client_from_url(config.to_owned()).await
            {
                panic!("Error detected on initialize queue config: {err}");
            };

            QUEUE_CONFIG
                .lock()
                .expect("Unable to initialize config")
                .replace(config);

            return;
        }

        panic!("Error detected on initialize queue config: config is disabled");
    }

    if config_path.is_none() {
        panic!("Error detected on initialize smtp config: config path is none");
    }

    QUEUE_CONFIG.lock().unwrap().replace(
        match QueueConfig::from_default_config_file(config_path.unwrap()) {
            Ok(config) => match config {
                OptionalConfig::Enabled(config) => config,
                OptionalConfig::Disabled => {
                    panic!("Error detected on initialize queue config: config is disabled")
                }
            },
            Err(err) => {
                panic!("Error detected on initialize queue config: {err}")
            }
        },
    );
}

pub(crate) async fn get_queue_config() -> QueueConfig {
    QUEUE_CONFIG
        .lock()
        .expect("Could not connect to the queue")
        .as_ref()
        .expect("Queue config not initialized")
        .to_owned()
}
