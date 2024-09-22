use myc_config::{load_config_from_file, optional_config::OptionalConfig};
use mycelium_base::utils::errors::{creation_err, MappedErrors};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TlsConfig {
    pub tls_cert_path: Option<String>,
    pub tls_key_path: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ApiConfig {
    pub service_ip: String,
    pub service_port: u16,
    pub allowed_origins: Vec<String>,
    pub service_workers: i32,
    pub gateway_timeout: u64,
    pub logging_level: String,
    pub routes: String,
    pub tls: OptionalConfig<TlsConfig>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TmpConfig {
    api: ApiConfig,
}

impl ApiConfig {
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
            Ok(config) => Ok(config.api),
            Err(err) => Err(err),
        }
    }
}
