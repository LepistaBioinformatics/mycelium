use crate::models::api_config::ApiConfig;

use myc_config::optional_config::OptionalConfig;
use myc_core::models::CoreConfig;
use myc_http_tools::models::auth_config::AuthConfig;
use myc_notifier::models::{QueueConfig, SmtpConfig};
use myc_prisma::models::PrismaConfig;
use mycelium_base::utils::errors::MappedErrors;
use std::path::PathBuf;

pub struct ConfigHandler {
    pub core: CoreConfig,
    pub prisma: PrismaConfig,
    pub api: ApiConfig,
    pub auth: AuthConfig,
    pub smtp: OptionalConfig<SmtpConfig>,
    pub queue: OptionalConfig<QueueConfig>,
}

impl ConfigHandler {
    pub fn init_from_file(file: PathBuf) -> Result<Self, MappedErrors> {
        Ok(Self {
            // Core configurations are used during the execution of the Mycelium
            // core functionalities, overall defined into use-cases layer.
            core: CoreConfig::from_default_config_file(file.clone())?,
            // Prisma configurations serves the prisma connector, which is
            // responsible for the communication with the database into the
            // adapters layer.
            prisma: PrismaConfig::from_default_config_file(file.clone())?,
            // API configuration should be used by the web server into the ports
            // layer.
            api: ApiConfig::from_default_config_file(file.clone())?,
            // Auth configuration should be used by the web server into the
            // ports.
            auth: AuthConfig::from_default_config_file(file.clone())?,
            // SMTP configuration should be used by the email sending repository
            // managements into the adapters layer.
            smtp: SmtpConfig::from_default_config_file(file.clone())?,
            // Queue configuration should be used by the queue repository
            // managements into the adapters layer.
            queue: QueueConfig::from_default_config_file(file.clone())?,
        })
    }
}
