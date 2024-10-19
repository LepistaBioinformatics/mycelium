use crate::modules::{
    AccountDeletionModule, AccountFetchingModule, AccountRegistrationModule,
    AccountTagDeletionModule, AccountTagRegistrationModule,
    AccountTagUpdatingModule, AccountUpdatingModule, ErrorCodeDeletionModule,
    ErrorCodeFetchingModule, ErrorCodeRegistrationModule,
    ErrorCodeUpdatingModule, GuestRoleDeletionModule, GuestRoleFetchingModule,
    GuestRoleRegistrationModule, GuestRoleUpdatingModule,
    GuestUserDeletionModule, GuestUserFetchingModule,
    GuestUserOnAccountUpdatingModule, GuestUserRegistrationModule,
    LicensedResourcesFetchingModule, MessageSendingQueueModule,
    ProfileFetchingModule, RoleDeletionModule, RoleFetchingModule,
    RoleRegistrationModule, RoleUpdatingModule, RoutesFetchingModule,
    TenantDeletionModule, TenantFetchingModule, TenantRegistrationModule,
    TenantTagDeletionModule, TenantTagRegistrationModule,
    TenantTagUpdatingModule, TenantUpdatingModule, TokenInvalidationModule,
    TokenRegistrationModule, UserDeletionModule, UserFetchingModule,
    UserRegistrationModule, UserUpdatingModule, WebHookDeletionModule,
    WebHookFetchingModule, WebHookRegistrationModule, WebHookUpdatingModule,
};

use actix_web::web;
use myc_mem_db::repositories::{
    RoutesFetchingMemDbRepo, RoutesFetchingMemDbRepoParameters,
};
use myc_notifier::repositories::{
    MessageSendingQueueRepository, MessageSendingQueueRepositoryParameters,
};
use myc_prisma::repositories::{
    AccountDeletionSqlDbRepository, AccountDeletionSqlDbRepositoryParameters,
    AccountFetchingSqlDbRepository, AccountFetchingSqlDbRepositoryParameters,
    AccountRegistrationSqlDbRepository,
    AccountRegistrationSqlDbRepositoryParameters,
    AccountTagDeletionSqlDbRepository,
    AccountTagDeletionSqlDbRepositoryParameters,
    AccountTagRegistrationSqlDbRepository,
    AccountTagRegistrationSqlDbRepositoryParameters,
    AccountTagUpdatingSqlDbRepository,
    AccountTagUpdatingSqlDbRepositoryParameters,
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
    TenantDeletionSqlDbRepository, TenantDeletionSqlDbRepositoryParameters,
    TenantFetchingSqlDbRepository, TenantFetchingSqlDbRepositoryParameters,
    TenantRegistrationSqlDbRepository,
    TenantRegistrationSqlDbRepositoryParameters,
    TenantTagDeletionSqlDbRepository,
    TenantTagDeletionSqlDbRepositoryParameters,
    TenantTagRegistrationSqlDbRepository,
    TenantTagRegistrationSqlDbRepositoryParameters,
    TenantTagUpdatingSqlDbRepository,
    TenantTagUpdatingSqlDbRepositoryParameters, TenantUpdatingSqlDbRepository,
    TenantUpdatingSqlDbRepositoryParameters, TokenInvalidationSqlDbRepository,
    TokenInvalidationSqlDbRepositoryParameters,
    TokenRegistrationSqlDbRepository,
    TokenRegistrationSqlDbRepositoryParameters, UserDeletionSqlDbRepository,
    UserDeletionSqlDbRepositoryParameters, UserFetchingSqlDbRepository,
    UserFetchingSqlDbRepositoryParameters, UserRegistrationSqlDbRepository,
    UserRegistrationSqlDbRepositoryParameters, UserUpdatingSqlDbRepository,
    UserUpdatingSqlDbRepositoryParameters, WebHookDeletionSqlDbRepository,
    WebHookDeletionSqlDbRepositoryParameters, WebHookFetchingSqlDbRepository,
    WebHookFetchingSqlDbRepositoryParameters,
    WebHookRegistrationSqlDbRepository,
    WebHookRegistrationSqlDbRepositoryParameters,
    WebHookUpdatingSqlDbRepository, WebHookUpdatingSqlDbRepositoryParameters,
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
        .app_data(Arc::new(
            AccountDeletionModule::builder()
                .with_component_parameters::<AccountDeletionSqlDbRepository>(
                    AccountDeletionSqlDbRepositoryParameters {}
                ).build()
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
            MessageSendingQueueModule::builder()
                .with_component_parameters::<MessageSendingQueueRepository>(
                    MessageSendingQueueRepositoryParameters {},
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
            TokenRegistrationModule::builder()
                .with_component_parameters::<TokenRegistrationSqlDbRepository>(
                    TokenRegistrationSqlDbRepositoryParameters {},
                )
                .build(),
        ))
        .app_data(Arc::new(
            TokenInvalidationModule::builder()
                .with_component_parameters::<TokenInvalidationSqlDbRepository>(
                    TokenInvalidationSqlDbRepositoryParameters {},
                )
                .build(),
        ))
        // ? -------------------------------------------------------------------
        // ? Account Tag
        // ? -------------------------------------------------------------------
        .app_data(Arc::new(
            AccountTagDeletionModule::builder()
                .with_component_parameters::<AccountTagDeletionSqlDbRepository>(
                    AccountTagDeletionSqlDbRepositoryParameters {},
                )
                .build(),
        ))
        .app_data(Arc::new(
            AccountTagRegistrationModule::builder()
                .with_component_parameters::<AccountTagRegistrationSqlDbRepository>(
                    AccountTagRegistrationSqlDbRepositoryParameters {},
                )
                .build(),
        ))
        .app_data(Arc::new(
            AccountTagUpdatingModule::builder()
                .with_component_parameters::<AccountTagUpdatingSqlDbRepository>(
                    AccountTagUpdatingSqlDbRepositoryParameters {},
                )
                .build(),
        ))
        // ? -------------------------------------------------------------------
        // ? Tenant
        // ? -------------------------------------------------------------------
        .app_data(Arc::new(
            TenantFetchingModule::builder()
                .with_component_parameters::<TenantFetchingSqlDbRepository>(
                    TenantFetchingSqlDbRepositoryParameters {},
                )
                .build(),
        ))
        .app_data(Arc::new(
            TenantRegistrationModule::builder()
                .with_component_parameters::<TenantRegistrationSqlDbRepository>(
                    TenantRegistrationSqlDbRepositoryParameters {},
                )
                .build(),
        ))
        .app_data(Arc::new(
            TenantUpdatingModule::builder()
                .with_component_parameters::<TenantUpdatingSqlDbRepository>(
                    TenantUpdatingSqlDbRepositoryParameters {},
                )
                .build(),
        ))
        .app_data(Arc::new(
            TenantDeletionModule::builder()
                .with_component_parameters::<TenantDeletionSqlDbRepository>(
                    TenantDeletionSqlDbRepositoryParameters {},
                )
                .build(),
        ))
        // ? -------------------------------------------------------------------
        // ? Tenant Tags
        // ? -------------------------------------------------------------------
        .app_data(Arc::new(
            TenantTagDeletionModule::builder()
                .with_component_parameters::<TenantTagDeletionSqlDbRepository>(
                    TenantTagDeletionSqlDbRepositoryParameters {},
                )
                .build(),
        ))
        .app_data(Arc::new(
            TenantTagRegistrationModule::builder()
                .with_component_parameters::<TenantTagRegistrationSqlDbRepository>(
                    TenantTagRegistrationSqlDbRepositoryParameters {},
                )
                .build(),
        ))
        .app_data(Arc::new(
            TenantTagUpdatingModule::builder()
                .with_component_parameters::<TenantTagUpdatingSqlDbRepository>(
                    TenantTagUpdatingSqlDbRepositoryParameters {},
                )
                .build(),
        ));
}
