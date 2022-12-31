use crate::modules::{
    AccountFetchingModule, AccountRegistrationModule,
    AccountTypeDeletionModule, AccountTypeRegistrationModule,
    AccountUpdatingModule, GuestRoleDeletionModule, GuestRoleFetchingModule,
    GuestRoleRegistrationModule, GuestRoleUpdatingModule,
    GuestUserRegistrationModule, MessageSendingModule, ProfileFetchingModule,
    RoleDeletionModule, RoleFetchingModule, RoleRegistrationModule,
    RoleUpdatingModule, TokenCleanupModule, TokenDeregistrationModule,
    TokenRegistrationModule, UserRegistrationModule, UserUpdatingModule,
};

use actix_web::web;
use myc_prisma::repositories::{
    AccountFetchingSqlDbRepository, AccountFetchingSqlDbRepositoryParameters,
    AccountRegistrationSqlDbRepository,
    AccountRegistrationSqlDbRepositoryParameters,
    AccountTypeDeletionSqlDbRepository,
    AccountTypeDeletionSqlDbRepositoryParameters,
    AccountTypeRegistrationSqlDbRepository,
    AccountTypeRegistrationSqlDbRepositoryParameters,
    AccountUpdatingSqlDbRepository, AccountUpdatingSqlDbRepositoryParameters,
    GuestRoleDeletionSqlDbRepository,
    GuestRoleDeletionSqlDbRepositoryParameters,
    GuestRoleFetchingSqlDbRepository,
    GuestRoleFetchingSqlDbRepositoryParameters,
    GuestRoleRegistrationSqlDbRepository,
    GuestRoleRegistrationSqlDbRepositoryParameters,
    GuestRoleUpdatingSqlDbRepository,
    GuestRoleUpdatingSqlDbRepositoryParameters,
    GuestUserRegistrationSqlDbRepository,
    GuestUserRegistrationSqlDbRepositoryParameters,
    ProfileFetchingSqlDbRepository, ProfileFetchingSqlDbRepositoryParameters,
    RoleDeletionSqlDbRepository, RoleDeletionSqlDbRepositoryParameters,
    RoleFetchingSqlDbRepository, RoleFetchingSqlDbRepositoryParameters,
    RoleRegistrationSqlDbRepository, RoleRegistrationSqlDbRepositoryParameters,
    RoleUpdatingSqlDbRepository, RoleUpdatingSqlDbRepositoryParameters,
    UserRegistrationSqlDbRepository, UserRegistrationSqlDbRepositoryParameters,
    UserUpdatingSqlDbRepository, UserUpdatingSqlDbRepositoryParameters,
};
use myc_redis::repositories::{
    TokenCleanupMemDbRepository, TokenCleanupMemDbRepositoryParameters,
    TokenDeregistrationMemDbRepository,
    TokenDeregistrationMemDbRepositoryParameters,
    TokenRegistrationMemDbRepository,
    TokenRegistrationMemDbRepositoryParameters,
};
use myc_smtp::repositories::{
    MessageSendingSqlDbRepository, MessageSendingSqlDbRepositoryParameters,
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
        // ? Account
        // ? -------------------------------------------------------------------
        .app_data(Arc::new(
            AccountFetchingModule::builder()
                .with_component_parameters::<AccountFetchingSqlDbRepository>(
                    AccountFetchingSqlDbRepositoryParameters {},
                )
                .build(),
        ))
        .app_data(Arc::new(
            AccountRegistrationModule::builder()
                .with_component_parameters::<AccountRegistrationSqlDbRepository>(
                    AccountRegistrationSqlDbRepositoryParameters {}
                ).build()
        ))
        .app_data(Arc::new(
            AccountUpdatingModule::builder()
                .with_component_parameters::<AccountUpdatingSqlDbRepository>(
                    AccountUpdatingSqlDbRepositoryParameters {}
                ).build()
        ))
        // ? -------------------------------------------------------------------
        // ? Account Type
        // ? -------------------------------------------------------------------
        .app_data(Arc::new(
            AccountTypeRegistrationModule::builder()
                .with_component_parameters::<AccountTypeRegistrationSqlDbRepository>(
                    AccountTypeRegistrationSqlDbRepositoryParameters {},
                )
                .build(),
        ))
        .app_data(Arc::new(
            AccountTypeDeletionModule::builder()
                .with_component_parameters::<AccountTypeDeletionSqlDbRepository>(
                    AccountTypeDeletionSqlDbRepositoryParameters {},
                )
                .build(),
        ))
        // ? -------------------------------------------------------------------
        // ? Guest User
        // ? -------------------------------------------------------------------
        .app_data(Arc::new(
            GuestUserRegistrationModule::builder()
                .with_component_parameters::<GuestUserRegistrationSqlDbRepository>(
                    GuestUserRegistrationSqlDbRepositoryParameters {},
                )
                .build(),
        ))
        // ? -------------------------------------------------------------------
        // ? Guest Role
        // ? -------------------------------------------------------------------
        .app_data(Arc::new(
            GuestRoleFetchingModule::builder()
                .with_component_parameters::<GuestRoleFetchingSqlDbRepository>(
                    GuestRoleFetchingSqlDbRepositoryParameters {},
                )
                .build(),
        ))
        .app_data(Arc::new(
            GuestRoleRegistrationModule::builder()
                .with_component_parameters::<GuestRoleRegistrationSqlDbRepository>(
                    GuestRoleRegistrationSqlDbRepositoryParameters {},
                )
                .build(),
        ))
        .app_data(Arc::new(
            GuestRoleDeletionModule::builder()
                .with_component_parameters::<GuestRoleDeletionSqlDbRepository>(
                    GuestRoleDeletionSqlDbRepositoryParameters {},
                )
                .build(),
        ))
        .app_data(Arc::new(
            GuestRoleUpdatingModule::builder()
                .with_component_parameters::<GuestRoleUpdatingSqlDbRepository>(
                    GuestRoleUpdatingSqlDbRepositoryParameters {},
                )
                .build(),
        ))
        // ? -------------------------------------------------------------------
        // ? Message sending
        // ? -------------------------------------------------------------------
        .app_data(Arc::new(
            MessageSendingModule::builder()
                .with_component_parameters::<MessageSendingSqlDbRepository>(
                    MessageSendingSqlDbRepositoryParameters {},
                )
                .build(),
        ))
        // ? -------------------------------------------------------------------
        // ? Profile
        // ? -------------------------------------------------------------------
        .app_data(Arc::new(
            ProfileFetchingModule::builder()
                .with_component_parameters::<ProfileFetchingSqlDbRepository>(
                    ProfileFetchingSqlDbRepositoryParameters {},
                )
                .build(),
        ))
        // ? -------------------------------------------------------------------
        // ? Role
        // ? -------------------------------------------------------------------
        .app_data(Arc::new(
            RoleRegistrationModule::builder()
                .with_component_parameters::<RoleRegistrationSqlDbRepository>(
                    RoleRegistrationSqlDbRepositoryParameters {}
                ).build()
        ))
        .app_data(Arc::new(
            RoleFetchingModule::builder()
                .with_component_parameters::<RoleFetchingSqlDbRepository>(
                    RoleFetchingSqlDbRepositoryParameters {}
                ).build()
        ))
        .app_data(Arc::new(
            RoleUpdatingModule::builder()
                .with_component_parameters::<RoleUpdatingSqlDbRepository>(
                    RoleUpdatingSqlDbRepositoryParameters {}
                ).build()
        ))
        .app_data(Arc::new(
            RoleDeletionModule::builder()
                .with_component_parameters::<RoleDeletionSqlDbRepository>(
                    RoleDeletionSqlDbRepositoryParameters {}
                ).build()
        ))
        // ? -------------------------------------------------------------------
        // ? User
        // ? -------------------------------------------------------------------
        .app_data(Arc::new(
            UserRegistrationModule::builder()
                .with_component_parameters::<UserRegistrationSqlDbRepository>(
                    UserRegistrationSqlDbRepositoryParameters {}
                ).build()
        ))
        .app_data(Arc::new(
            UserUpdatingModule::builder()
                .with_component_parameters::<UserUpdatingSqlDbRepository>(
                    UserUpdatingSqlDbRepositoryParameters {}
                ).build()
        ))
        // ? -------------------------------------------------------------------
        // ? Token
        // ? -------------------------------------------------------------------
        .app_data(Arc::new(
            TokenRegistrationModule::builder()
                .with_component_parameters::<TokenRegistrationMemDbRepository>(
                    TokenRegistrationMemDbRepositoryParameters {}
                ).build()
        ))
        .app_data(Arc::new(
            TokenDeregistrationModule::builder()
                .with_component_parameters::<TokenDeregistrationMemDbRepository>(
                    TokenDeregistrationMemDbRepositoryParameters {}
                ).build()
        ))
        .app_data(Arc::new(
            TokenCleanupModule::builder()
                .with_component_parameters::<TokenCleanupMemDbRepository>(
                    TokenCleanupMemDbRepositoryParameters {}
                ).build()
        ));
}
