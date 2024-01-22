use crate::domain::dtos::session_token::TokenSecret;

use myc_config::load_config_from_file;
use mycelium_base::utils::errors::{creation_err, MappedErrors};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoreConfig {
    pub token: TokenSecret,
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
            Ok(config) => Ok(config.core),
            Err(err) => Err(err),
        }
    }
}
