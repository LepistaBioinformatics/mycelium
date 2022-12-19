use std::{env::var_os, sync::Arc};

use myc::adapters::repositories::sql_db::service::profile_fetching::{
    ProfileFetchingSqlDbRepository, ProfileFetchingSqlDbRepositoryParameters,
};

use crate::modules::service::ProfileFetchingModule;

pub struct SvcConfig {
    pub service_ip: String,
    pub service_port: u16,
    pub allowed_origins: Vec<String>,
}

impl SvcConfig {
    pub fn new() -> Self {
        Self {
            service_ip: match var_os("SERVICE_IP") {
                Some(path) => path.into_string().unwrap(),
                None => String::from("0.0.0.0"),
            },
            service_port: match var_os("SERVICE_PORT") {
                Some(path) => {
                    path.into_string().unwrap().parse::<u16>().unwrap()
                }
                None => 8080,
            },
            allowed_origins: match var_os("ALLOWED_ORIGINS") {
                Some(path) => path
                    .into_string()
                    .unwrap()
                    .split(",")
                    .into_iter()
                    .map(|i| i.to_string())
                    .collect(),
                None => vec!["http://localhost:3000".to_string()],
            },
        }
    }
}

pub struct InjectableModulesConfig {
    pub profile_fetching_module: Arc<ProfileFetchingModule>,
}

impl InjectableModulesConfig {
    pub async fn new() -> InjectableModulesConfig {
        // ? -------------------------------------------------------------------
        // ? Customer registration
        // ? -------------------------------------------------------------------

        let profile_fetching_module = Arc::new(
            ProfileFetchingModule::builder()
                .with_component_parameters::<ProfileFetchingSqlDbRepository>(
                    ProfileFetchingSqlDbRepositoryParameters {},
                )
                .build(),
        );

        // ? -------------------------------------------------------------------
        // ? Return `PrismaClientConfig` type
        // ? -------------------------------------------------------------------

        InjectableModulesConfig {
            profile_fetching_module,
        }
    }
}
