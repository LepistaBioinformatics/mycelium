use std::path::PathBuf;

use crate::providers::{
    azure_config::AzureOauthConfig, google_config::GoogleOauthConfig,
};

use clean_base::utils::errors::{factories::creation_err, MappedErrors};
use myc_config::{load_config_from_file, optional_config::OptionalConfig};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthConfig {
    pub internal: OptionalConfig<bool>,
    pub google: OptionalConfig<GoogleOauthConfig>,
    pub azure: OptionalConfig<AzureOauthConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct TmpConfig {
    auth: AuthConfig,
}

impl AuthConfig {
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
            Ok(config) => Ok(config.auth),
            Err(err) => Err(err),
        }
    }
}
