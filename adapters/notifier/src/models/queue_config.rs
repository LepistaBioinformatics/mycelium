use super::tmp_config::TmpConfig;

use myc_config::{load_config_from_file, secret_resolver::SecretResolver};
use mycelium_base::utils::errors::{creation_err, MappedErrors};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueConfig {
    pub protocol: SecretResolver<String>,
    pub hostname: SecretResolver<String>,
    pub password: SecretResolver<String>,
    pub email_queue_name: SecretResolver<String>,
    pub consume_interval_in_secs: SecretResolver<u64>,
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
