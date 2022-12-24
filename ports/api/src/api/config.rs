use crate::modules::{
    manager::{
        AccountFetchingModule, GuestUserRegistrationModule,
        MessageSendingModule,
    },
    service::ProfileFetchingModule,
};
use actix_web::web;
use myc_core::adapters::{
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
use std::{env::var_os, sync::Arc};

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

/// Configure injection modules.
pub fn configure(config: &mut web::ServiceConfig) {
    config
        // ? -------------------------------------------------------------------
        // ? Profile fetching repo
        // ? -------------------------------------------------------------------
        .app_data(Arc::new(
            ProfileFetchingModule::builder()
                .with_component_parameters::<ProfileFetchingSqlDbRepository>(
                    ProfileFetchingSqlDbRepositoryParameters {},
                )
                .build(),
        ))
        // ? -------------------------------------------------------------------
        // ? Account fetching repo
        // ? -------------------------------------------------------------------
        .app_data(Arc::new(
            AccountFetchingModule::builder()
                .with_component_parameters::<AccountFetchingSqlDbRepository>(
                    AccountFetchingSqlDbRepositoryParameters {},
                )
                .build(),
        ))
        // ? -------------------------------------------------------------------
        // ? Guest User registration repo
        // ? -------------------------------------------------------------------
        .app_data(Arc::new(
            GuestUserRegistrationModule::builder()
                .with_component_parameters::<GuestUserRegistrationSqlDbRepository>(
                    GuestUserRegistrationSqlDbRepositoryParameters {},
                )
                .build(),
        ))
        // ? -------------------------------------------------------------------
        // ? Message sending repo
        // ? -------------------------------------------------------------------
        .app_data(Arc::new(
            MessageSendingModule::builder()
                .with_component_parameters::<MessageSendingSqlDbRepository>(
                    MessageSendingSqlDbRepositoryParameters {},
                )
                .build(),
        ));
}
