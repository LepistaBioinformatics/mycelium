use clean_base::utils::errors::{factories::creation_err, MappedErrors};
use myc_config::{load_config_from_file, optional_config::OptionalConfig};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SmtpConfig {
    pub host: String,
    pub username: String,
    pub password: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TmpConfig {
    smtp: OptionalConfig<SmtpConfig>,
}

impl SmtpConfig {
    pub fn from_default_config_file(
        file: PathBuf,
    ) -> Result<OptionalConfig<Self>, MappedErrors> {
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
