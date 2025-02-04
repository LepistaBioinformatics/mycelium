use lettre::SmtpTransport;
use myc_config::{load_config_from_file, secret_resolver::SecretResolver};
use mycelium_base::utils::errors::{creation_err, MappedErrors};
use redis::Client;
use serde::Deserialize;
use shaku::Interface;
use std::{path::PathBuf, sync::Arc};

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SmtpConfig {
    pub host: SecretResolver<String>,
    pub username: SecretResolver<String>,
    pub password: SecretResolver<String>,
}

unsafe impl Send for SmtpConfig {}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueConfig {
    pub protocol: SecretResolver<String>,
    pub hostname: SecretResolver<String>,
    pub password: SecretResolver<String>,
    pub email_queue_name: SecretResolver<String>,
    pub consume_interval_in_secs: SecretResolver<u64>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TmpConfig {
    smtp: SmtpConfig,
    queue: QueueConfig,
}

impl SmtpConfig {
    pub fn from_default_config_file(
        file: PathBuf,
    ) -> Result<Self, MappedErrors> {
        if !file.exists() {
            return creation_err(format!(
                "Could not find config file: {}",
                file.to_str().unwrap()
            ))
            .as_error();
        }

        match load_config_from_file::<TmpConfig>(file) {
            Ok(config) => Ok(config.smtp),
            Err(err) => Err(err),
        }
    }
}

impl QueueConfig {
    pub fn from_default_config_file(
        file: PathBuf,
    ) -> Result<Self, MappedErrors> {
        if !file.exists() {
            return creation_err(format!(
                "Could not find config file: {}",
                file.to_str().unwrap()
            ))
            .as_error();
        }

        match load_config_from_file::<TmpConfig>(file) {
            Ok(config) => Ok(config.queue),
            Err(err) => Err(err),
        }
    }
}

pub trait ClientProvider: Interface + Send + Sync {
    fn get_queue_client(&self) -> Arc<Client>;
    fn get_smtp_client(&self) -> Arc<SmtpTransport>;
    fn get_config(&self) -> Arc<QueueConfig>;
}
