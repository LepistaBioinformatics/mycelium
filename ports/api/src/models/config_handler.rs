use super::api_config::ApiConfig;

#[cfg(feature = "postgres-backend")]
use myc_adapters_shared_lib::models::RedisConfig;
#[cfg(feature = "postgres-backend")]
use myc_config::{optional_config::OptionalConfig, VaultConfig};
use myc_core::models::CoreConfig;
#[cfg(feature = "postgres-backend")]
use myc_diesel::models::config::DieselConfig;
#[cfg(feature = "standalone")]
use myc_diesel_sqlite::config::SqliteConfig;
use myc_http_tools::models::auth_config::AuthConfig;
#[cfg(feature = "postgres-backend")]
use myc_notifier::models::{QueueConfig, SmtpConfig};
use mycelium_base::utils::errors::MappedErrors;
use std::path::PathBuf;

#[derive(Clone)]
pub struct ConfigHandler {
    pub core: CoreConfig,
    pub api: ApiConfig,
    pub auth: AuthConfig,

    #[cfg(feature = "postgres-backend")]
    pub diesel: DieselConfig,
    #[cfg(feature = "postgres-backend")]
    pub smtp: SmtpConfig,
    #[cfg(feature = "postgres-backend")]
    pub queue: QueueConfig,
    #[cfg(feature = "postgres-backend")]
    pub redis: RedisConfig,
    #[cfg(feature = "postgres-backend")]
    pub vault: OptionalConfig<VaultConfig>,

    #[cfg(feature = "standalone")]
    pub sqlite: SqliteConfig,
}

impl ConfigHandler {
    pub fn init_from_file(file: PathBuf) -> Result<Self, MappedErrors> {
        Ok(Self {
            // Core configurations are used during the execution of the Mycelium
            // core functionalities, overall defined into use-cases layer.
            core: CoreConfig::from_default_config_file(file.clone())?,
            // API configuration should be used by the web server into the ports
            // layer.
            api: ApiConfig::from_default_config_file(file.clone())?,
            // Auth configuration should be used by the web server into the
            // ports.
            auth: AuthConfig::from_default_config_file(file.clone())?,
            // Database configurations serves the database connector, which is
            // responsible for the communication with the database into the
            // adapters layer.
            #[cfg(feature = "postgres-backend")]
            diesel: DieselConfig::from_default_config_file(file.clone())?,
            // SMTP configuration should be used by the email sending repository
            // managements into the adapters layer.
            #[cfg(feature = "postgres-backend")]
            smtp: SmtpConfig::from_default_config_file(file.clone())?,
            // Queue configuration should be used by the queue repository
            // managements into the adapters layer.
            #[cfg(feature = "postgres-backend")]
            queue: QueueConfig::from_default_config_file(file.clone())?,
            // Redis configuration should be used by the redis repository
            // managements into the adapters layer.
            #[cfg(feature = "postgres-backend")]
            redis: RedisConfig::from_default_config_file(file.clone())?,
            // Vault configuration should be used by the secret resolver into
            // the domain layer.
            #[cfg(feature = "postgres-backend")]
            vault: VaultConfig::from_default_config_file(file.clone())?,
            // SQLite configuration points at the standalone database file.
            #[cfg(feature = "standalone")]
            sqlite: SqliteConfig::from_default_config_file(file.clone())?,
        })
    }
}
