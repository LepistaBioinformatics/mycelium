use myc_config::{
    load_config_from_file, optional_config::OptionalConfig,
    secret_resolver::SecretResolver,
};
use myc_core::domain::dtos::http::Protocol;
use mycelium_base::utils::errors::{creation_err, MappedErrors};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TlsConfig {
    pub tls_cert: SecretResolver<String>,
    pub tls_key: SecretResolver<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LogFormat {
    /// ANSI format
    ///
    /// This format is human-readable and colorful.
    Ansi,

    /// YAML format
    ///
    /// This format is machine-readable and can be used for log analysis.
    Jsonl,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LoggingTarget {
    Stdout,
    File {
        path: String,
    },
    Jaeger {
        name: String,
        protocol: Protocol,
        host: String,
        port: u32,
    },
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoggingConfig {
    pub level: String,
    pub format: LogFormat,
    pub target: Option<LoggingTarget>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheConfig {
    /// JWKS TTL
    ///
    /// The time to live for the JWKS cache.
    pub jwks_ttl: Option<u64>,

    /// Email TTL
    ///
    /// The time to live for the email cache.
    pub email_ttl: Option<u64>,

    /// Profile TTL
    ///
    /// The time to live for the profile cache.
    pub profile_ttl: Option<u64>,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            jwks_ttl: Some(60 * 60 * 12), // 12 hours
            email_ttl: Some(60 * 10),     // 10 minutes
            profile_ttl: Some(60 * 10),   // 10 minutes
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiConfig {
    pub service_ip: String,
    pub service_port: u16,
    pub allowed_origins: Vec<String>,
    pub service_workers: i32,
    pub gateway_timeout: u64,
    pub logging: LoggingConfig,
    pub routes: Option<String>,
    pub tls: OptionalConfig<TlsConfig>,
    pub cache: Option<CacheConfig>,
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
