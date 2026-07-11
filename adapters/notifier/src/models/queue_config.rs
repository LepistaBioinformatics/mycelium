use myc_config::{load_config_from_file, secret_resolver::SecretResolver};
use mycelium_base::utils::errors::{creation_err, MappedErrors};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueConfig {
    #[serde(default = "default_email_queue_name")]
    pub email_queue_name: SecretResolver<String>,
    #[serde(default = "default_consume_interval_in_secs")]
    pub consume_interval_in_secs: SecretResolver<u64>,
}

fn default_email_queue_name() -> SecretResolver<String> {
    SecretResolver::Value("emails".to_string())
}

fn default_consume_interval_in_secs() -> SecretResolver<u64> {
    SecretResolver::Value(15)
}

// Loaded on its own -- not bundled with `SmtpConfig` -- so standalone mode
// (which has no `[smtp]` section) can still load `[queue]`, the polling
// config the email dispatcher needs regardless of backend.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TmpConfig {
    queue: QueueConfig,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queue_config_defaults_when_fields_absent() {
        let config: QueueConfig = toml::from_str("").unwrap();

        assert_eq!(
            config.email_queue_name,
            SecretResolver::Value("emails".to_string())
        );
        assert_eq!(config.consume_interval_in_secs, SecretResolver::Value(15));
    }
}
