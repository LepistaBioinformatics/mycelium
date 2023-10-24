use std::path::PathBuf;

use clean_base::utils::errors::{factories::creation_err, MappedErrors};
use myc_config::load_config_from_file;
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SmtpConfig {
    pub enable: bool,
    pub host: String,
    pub username: String,
    pub password: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TmpConfig {
    smtp: SmtpConfig,
}

impl SmtpConfig {
    pub async fn from_default_config_file(
        file: PathBuf,
    ) -> Result<Self, MappedErrors> {
        if !file.exists() {
            return creation_err(format!(
                "Could not find config file: {}",
                file.to_str().unwrap()
            ))
            .as_error();
        }

        match load_config_from_file::<TmpConfig>(file).await {
            Ok(config) => Ok(config.smtp),
            Err(err) => Err(err),
        }
    }
}
