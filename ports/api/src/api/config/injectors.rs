use crate::modules::{
    AccountFetchingModule, AccountRegistrationModule,
    AccountTypeDeletionModule, AccountTypeRegistrationModule,
    AccountUpdatingModule, ErrorCodeDeletionModule, ErrorCodeFetchingModule,
    ErrorCodeRegistrationModule, ErrorCodeUpdatingModule,
    GuestRoleDeletionModule, GuestRoleFetchingModule,
    GuestRoleRegistrationModule, GuestRoleUpdatingModule,
    GuestUserDeletionModule, GuestUserFetchingModule,
    GuestUserOnAccountUpdatingModule, GuestUserRegistrationModule,
    LicensedResourcesFetchingModule, MessageSendingModule,
    ProfileFetchingModule, RoleDeletionModule, RoleFetchingModule,
    RoleRegistrationModule, RoleUpdatingModule, RoutesFetchingModule,
    SessionTokenDeletionModule, SessionTokenFetchingModule,
    SessionTokenRegistrationModule, UserDeletionModule, UserFetchingModule,
    UserRegistrationModule, UserUpdatingModule, WebHookDeletionModule,
    WebHookFetchingModule, WebHookRegistrationModule, WebHookUpdatingModule,
};

use actix_web::web;
use myc_mem_db::repositories::{
    RoutesFetchingMemDbRepo, RoutesFetchingMemDbRepoParameters,
};
use myc_prisma::repositories::{
    AccountFetchingSqlDbRepository, AccountFetchingSqlDbRepositoryParameters,
    AccountRegistrationSqlDbRepository,
    AccountRegistrationSqlDbRepositoryParameters,
    AccountTypeDeletionSqlDbRepository,
    AccountTypeDeletionSqlDbRepositoryParameters,
    AccountTypeRegistrationSqlDbRepository,
    AccountTypeRegistrationSqlDbRepositoryParameters,
    AccountUpdatingSqlDbRepository, AccountUpdatingSqlDbRepositoryParameters,
    ErrorCodeDeletionDeletionSqlDbRepository,
    ErrorCodeDeletionDeletionSqlDbRepositoryParameters,
    ErrorCodeFetchingSqlDbRepository,
    ErrorCodeFetchingSqlDbRepositoryParameters,
    ErrorCodeRegistrationSqlDbRepository,
    ErrorCodeRegistrationSqlDbRepositoryParameters,
    ErrorCodeUpdatingSqlDbRepository,
    ErrorCodeUpdatingSqlDbRepositoryParameters,
    GuestRoleDeletionSqlDbRepository,
    GuestRoleDeletionSqlDbRepositoryParameters,
    GuestRoleFetchingSqlDbRepository,
    GuestRoleFetchingSqlDbRepositoryParameters,
    GuestRoleRegistrationSqlDbRepository,
    GuestRoleRegistrationSqlDbRepositoryParameters,
    GuestRoleUpdatingSqlDbRepository,
    GuestRoleUpdatingSqlDbRepositoryParameters,
    GuestUserDeletionSqlDbRepository,
    GuestUserDeletionSqlDbRepositoryParameters,
    GuestUserFetchingSqlDbRepository,
    GuestUserFetchingSqlDbRepositoryParameters,
    GuestUserOnAccountUpdatingSqlDbRepository,
    GuestUserOnAccountUpdatingSqlDbRepositoryParameters,
    GuestUserRegistrationSqlDbRepository,
    GuestUserRegistrationSqlDbRepositoryParameters,
    LicensedResourcesFetchingSqlDbRepository,
    LicensedResourcesFetchingSqlDbRepositoryParameters,
    ProfileFetchingSqlDbRepository, ProfileFetchingSqlDbRepositoryParameters,
    RoleDeletionSqlDbRepository, RoleDeletionSqlDbRepositoryParameters,
    RoleFetchingSqlDbRepository, RoleFetchingSqlDbRepositoryParameters,
    RoleRegistrationSqlDbRepository, RoleRegistrationSqlDbRepositoryParameters,
    RoleUpdatingSqlDbRepository, RoleUpdatingSqlDbRepositoryParameters,
    SessionTokenDeletionSqlDbRepository,
    SessionTokenDeletionSqlDbRepositoryParameters,
    SessionTokenFetchingSqlDbRepository,
    SessionTokenFetchingSqlDbRepositoryParameters,
    SessionTokenRegistrationSqlDbRepository,
    SessionTokenRegistrationSqlDbRepositoryParameters,
    UserDeletionSqlDbRepository, UserDeletionSqlDbRepositoryParameters,
    UserFetchingSqlDbRepository, UserFetchingSqlDbRepositoryParameters,
    UserRegistrationSqlDbRepository, UserRegistrationSqlDbRepositoryParameters,
    UserUpdatingSqlDbRepository, UserUpdatingSqlDbRepositoryParameters,
    WebHookDeletionSqlDbRepository, WebHookDeletionSqlDbRepositoryParameters,
    WebHookFetchingSqlDbRepository, WebHookFetchingSqlDbRepositoryParameters,
    WebHookRegistrationSqlDbRepository,
    WebHookRegistrationSqlDbRepositoryParameters,
    WebHookUpdatingSqlDbRepository, WebHookUpdatingSqlDbRepositoryParameters,
};
use myc_smtp::repositories::{
    MessageSendingSmtpRepository, MessageSendingSmtpRepositoryParameters,
};
use std::sync::Arc;

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
            GuestUserFetchingModule::builder()
                .with_component_parameters::<GuestUserFetchingSqlDbRepository>(
                    GuestUserFetchingSqlDbRepositoryParameters {},
                )
                .build(),
        ))
        .app_data(Arc::new(
            GuestUserRegistrationModule::builder()
                .with_component_parameters::<GuestUserRegistrationSqlDbRepository>(
                    GuestUserRegistrationSqlDbRepositoryParameters {},
                )
                .build(),
        ))
        .app_data(Arc::new(
            GuestUserOnAccountUpdatingModule::builder()
                .with_component_parameters::<GuestUserOnAccountUpdatingSqlDbRepository>(
                    GuestUserOnAccountUpdatingSqlDbRepositoryParameters {},
                )
                .build(),
        ))
        .app_data(Arc::new(
            GuestUserDeletionModule::builder()
                .with_component_parameters::<GuestUserDeletionSqlDbRepository>(
                    GuestUserDeletionSqlDbRepositoryParameters {},
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
                .with_component_parameters::<MessageSendingSmtpRepository>(
                    MessageSendingSmtpRepositoryParameters {},
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
        .app_data(Arc::new(
            UserFetchingModule::builder()
                .with_component_parameters::<UserFetchingSqlDbRepository>(
                    UserFetchingSqlDbRepositoryParameters {}
                ).build()
        ))
        .app_data(Arc::new(
            UserDeletionModule::builder()
                .with_component_parameters::<UserDeletionSqlDbRepository>(
                    UserDeletionSqlDbRepositoryParameters {}
                ).build()
        ))
        // ? -------------------------------------------------------------------
        // ? LicensedResources
        // ? -------------------------------------------------------------------
        .app_data(Arc::new(
            LicensedResourcesFetchingModule::builder()
                .with_component_parameters::<LicensedResourcesFetchingSqlDbRepository>(
                    LicensedResourcesFetchingSqlDbRepositoryParameters {}
                ).build()
        ))
        // ? -------------------------------------------------------------------
        // ? Routes
        // ? -------------------------------------------------------------------
        .app_data(Arc::new(
            RoutesFetchingModule::builder()
                .with_component_parameters::<RoutesFetchingMemDbRepo>(
                    RoutesFetchingMemDbRepoParameters {},
                )
                .build(),
        ))
        // ? -------------------------------------------------------------------
        // ? ErrorCodes
        // ? -------------------------------------------------------------------
        .app_data(Arc::new(
            ErrorCodeDeletionModule::builder()
                .with_component_parameters::<ErrorCodeDeletionDeletionSqlDbRepository>(
                    ErrorCodeDeletionDeletionSqlDbRepositoryParameters {},
                )
                .build(),
        ))
        .app_data(Arc::new(
            ErrorCodeFetchingModule::builder()
                .with_component_parameters::<ErrorCodeFetchingSqlDbRepository>(
                    ErrorCodeFetchingSqlDbRepositoryParameters {},
                )
                .build(),
        ))
        .app_data(Arc::new(
            ErrorCodeRegistrationModule::builder()
                .with_component_parameters::<ErrorCodeRegistrationSqlDbRepository>(
                    ErrorCodeRegistrationSqlDbRepositoryParameters {},
                )
                .build(),
        ))
        .app_data(Arc::new(
            ErrorCodeUpdatingModule::builder()
                .with_component_parameters::<ErrorCodeUpdatingSqlDbRepository>(
                    ErrorCodeUpdatingSqlDbRepositoryParameters {},
                )
                .build(),
        ))
        // ? -------------------------------------------------------------------
        // ? ErrorCodes
        // ? -------------------------------------------------------------------
        .app_data(Arc::new(
            WebHookRegistrationModule::builder()
                .with_component_parameters::<WebHookRegistrationSqlDbRepository>(
                    WebHookRegistrationSqlDbRepositoryParameters {},
                )
                .build(),
        ))
        .app_data(Arc::new(
            WebHookDeletionModule::builder()
                .with_component_parameters::<WebHookDeletionSqlDbRepository>(
                    WebHookDeletionSqlDbRepositoryParameters {},
                )
                .build(),
        ))
        .app_data(Arc::new(
            WebHookFetchingModule::builder()
                .with_component_parameters::<WebHookFetchingSqlDbRepository>(
                    WebHookFetchingSqlDbRepositoryParameters {},
                )
                .build(),
        ))
        .app_data(Arc::new(
            WebHookUpdatingModule::builder()
                .with_component_parameters::<WebHookUpdatingSqlDbRepository>(
                    WebHookUpdatingSqlDbRepositoryParameters {},
                )
                .build(),
        ))
        // ? -------------------------------------------------------------------
        // ? SessionTokens
        // ? -------------------------------------------------------------------
        .app_data(Arc::new(
            SessionTokenRegistrationModule::builder()
                .with_component_parameters::<SessionTokenRegistrationSqlDbRepository>(
                    SessionTokenRegistrationSqlDbRepositoryParameters {},
                )
                .build(),
        ))
        .app_data(Arc::new(
            SessionTokenFetchingModule::builder()
                .with_component_parameters::<SessionTokenFetchingSqlDbRepository>(
                    SessionTokenFetchingSqlDbRepositoryParameters {},
                )
                .build(),
        ))
        .app_data(Arc::new(
            SessionTokenDeletionModule::builder()
                .with_component_parameters::<SessionTokenDeletionSqlDbRepository>(
                    SessionTokenDeletionSqlDbRepositoryParameters {},
                )
                .build(),
        ));
}
