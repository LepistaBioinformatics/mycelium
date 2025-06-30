use super::{
    external_providers_config::ExternalProviderConfig,
    internal_auth_config::InternalOauthConfig,
};

use myc_config::{load_config_from_file, optional_config::OptionalConfig};
use mycelium_base::utils::errors::{creation_err, MappedErrors};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthConfig {
    pub internal: OptionalConfig<InternalOauthConfig>,
    pub external: OptionalConfig<Vec<ExternalProviderConfig>>,
}

#[derive(Clone, Debug, Deserialize)]
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
