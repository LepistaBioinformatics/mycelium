use crate::{
    optional_config::OptionalConfig, secret_resolver::SecretResolver,
    use_cases::load_config_from_file,
};

use mycelium_base::utils::errors::{creation_err, MappedErrors};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultConfig {
    pub url: String,
    pub token: SecretResolver<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TmpConfig {
    vault: OptionalConfig<VaultConfig>,
}

impl VaultConfig {
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
            Ok(config) => {
                match config.vault.to_owned() {
                    OptionalConfig::Enabled(vault) => match vault.token {
                        SecretResolver::Vault { path, key } => {
                            return creation_err(format!(
                                "Vault token cannot be resolved from vault. \
                                Please provide the token directly if the \
                                environment is DEV or set an environment \
                                variable to resolve the token in production.: \
                                {path}/{key}",
                                path = path,
                                key = key
                            ))
                            .as_error();
                        }
                        _ => {}
                    },
                    _ => {}
                }

                Ok(config.vault)
            }
            Err(err) => Err(err),
        }
    }
}
