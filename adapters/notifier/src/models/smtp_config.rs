use myc_config::{load_config_from_file, secret_resolver::SecretResolver};
use mycelium_base::utils::errors::{creation_err, MappedErrors};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SmtpConfig {
    pub host: SecretResolver<String>,
    pub username: SecretResolver<String>,
    pub password: SecretResolver<String>,
    pub port: SecretResolver<u16>,
}

unsafe impl Send for SmtpConfig {}

// Loaded on its own -- not bundled with `QueueConfig` -- so postgres-backend
// (the only mode where SMTP applies) doesn't force standalone's `[queue]`
// loader to also require a `[smtp]` section it has no use for.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TmpConfig {
    smtp: SmtpConfig,
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
