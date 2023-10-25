use crate::models::api_config::ApiConfig;

use clean_base::utils::errors::MappedErrors;
use myc_config::optional_config::OptionalConfig;
use myc_core::models::CoreConfig;
use myc_http_tools::models::auth_config::AuthConfig;
use myc_prisma::models::PrismaConfig;
use myc_smtp::models::SmtpConfig;
use std::path::PathBuf;

pub(crate) struct ConfigHandler {
    pub core: CoreConfig,
    pub prisma: PrismaConfig,
    pub api: ApiConfig,
    pub auth: AuthConfig,
    pub smtp: OptionalConfig<SmtpConfig>,
}

impl ConfigHandler {
    pub fn init_from_file(file: PathBuf) -> Result<Self, MappedErrors> {
        // Core configurations are used during the execution of the Mycelium
        // core functionalities, overall defined into use-cases layer.
        let core_config = CoreConfig::from_default_config_file(file.clone())?;

        // Prisma configurations serves the prisma connector, which is
        // responsible for the communication with the database into the adapters
        // layer.
        let prisma_config =
            PrismaConfig::from_default_config_file(file.clone())?;

        // SMTP configuration should be used by the email sending repository
        // managements into the adapters layer.
        let smtp_config = SmtpConfig::from_default_config_file(file.clone())?;

        // API configuration should be used by the web server into the ports
        // layer.
        let api_config = ApiConfig::from_default_config_file(file.clone())?;

        Ok(Self {
            core: core_config,
            prisma: prisma_config,
            smtp: smtp_config,
            api: api_config,
            auth: AuthConfig {
                internal: OptionalConfig::Disabled,
                google: OptionalConfig::Disabled,
                azure: OptionalConfig::Disabled,
            },
        })
    }
}
