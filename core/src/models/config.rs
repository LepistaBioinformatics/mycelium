use super::{
    account_life_cycle_config::AccountLifeCycle, ProfileSigningConfig,
    WebhookConfig,
};

use myc_config::load_config_from_file;
use mycelium_base::utils::errors::{creation_err, MappedErrors};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoreConfig {
    pub account_life_cycle: AccountLifeCycle,
    pub webhook: WebhookConfig,

    /// Downstream profile signing; absent keeps signing disabled
    #[serde(default)]
    pub profile_signing: Option<ProfileSigningConfig>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TmpConfig {
    core: CoreConfig,
}

impl CoreConfig {
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
            Ok(config) => {
                config.core.account_life_cycle.validate_hmac_config()?;

                if let Some(signing) = &config.core.profile_signing {
                    signing.validate()?;
                }

                Ok(config.core)
            }
            Err(err) => Err(err),
        }
    }
}
