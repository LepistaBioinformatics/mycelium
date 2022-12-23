use std::{env::var_os, sync::Arc};

use myc::adapters::{
    repositories::sql_db::{
        manager::guest_user_registration::{
            GuestUserRegistrationSqlDbRepository,
            GuestUserRegistrationSqlDbRepositoryParameters,
        },
        service::profile_fetching::{
            ProfileFetchingSqlDbRepository,
            ProfileFetchingSqlDbRepositoryParameters,
        },
        shared::account_fetching::{
            AccountFetchingSqlDbRepository,
            AccountFetchingSqlDbRepositoryParameters,
        },
    },
    smtp::message_sending::{
        MessageSendingSqlDbRepository, MessageSendingSqlDbRepositoryParameters,
    },
};

use crate::modules::{
    manager::{
        AccountFetchingModule, GuestUserRegistrationModule,
        MessageSendingModule,
    },
    service::ProfileFetchingModule,
};

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
    pub account_fetching_module: Arc<AccountFetchingModule>,
    pub guest_user_registration_module: Arc<GuestUserRegistrationModule>,
    pub message_sending_module: Arc<MessageSendingModule>,
}

impl InjectableModulesConfig {
    pub async fn new() -> InjectableModulesConfig {
        // ? -------------------------------------------------------------------
        // ? Profile fetching
        // ? -------------------------------------------------------------------

        let profile_fetching_module = Arc::new(
            ProfileFetchingModule::builder()
                .with_component_parameters::<ProfileFetchingSqlDbRepository>(
                    ProfileFetchingSqlDbRepositoryParameters {},
                )
                .build(),
        );

        // ? -------------------------------------------------------------------
        // ? Account fetching
        // ? -------------------------------------------------------------------

        let account_fetching_module = Arc::new(
            AccountFetchingModule::builder()
                .with_component_parameters::<AccountFetchingSqlDbRepository>(
                    AccountFetchingSqlDbRepositoryParameters {},
                )
                .build(),
        );

        // ? -------------------------------------------------------------------
        // ? GuestUser registration
        // ? -------------------------------------------------------------------

        let guest_user_registration_module = Arc::new(
            GuestUserRegistrationModule::builder()
                .with_component_parameters::<GuestUserRegistrationSqlDbRepository>(
                    GuestUserRegistrationSqlDbRepositoryParameters {},
                )
                .build(),
        );

        // ? -------------------------------------------------------------------
        // ? Message registration
        // ? -------------------------------------------------------------------

        let message_sending_module = Arc::new(
            MessageSendingModule::builder()
                .with_component_parameters::<MessageSendingSqlDbRepository>(
                    MessageSendingSqlDbRepositoryParameters {},
                )
                .build(),
        );

        // ? -------------------------------------------------------------------
        // ? Return `PrismaClientConfig` type
        // ? -------------------------------------------------------------------

        InjectableModulesConfig {
            profile_fetching_module,
            account_fetching_module,
            guest_user_registration_module,
            message_sending_module,
        }
    }
}
