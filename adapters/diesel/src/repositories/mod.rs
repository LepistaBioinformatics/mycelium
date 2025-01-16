use shaku::module;

mod account;
mod account_tag;
mod config;
mod error_code;
mod guest_role;
mod guest_user;
mod licensed_resources;
mod profile;
mod tenant;
mod tenant_tag;
mod token;
mod user;
mod webhook;

use account::*;
use account_tag::*;
use error_code::*;
use guest_role::*;
use guest_user::*;
use licensed_resources::*;
use profile::*;
use tenant::*;
use tenant_tag::*;
use token::*;
use user::*;
use webhook::*;

pub use config::*;

module! {
    pub AppModule {
        components = [
            //
            // Provide the database pool
            //
            DieselDbPoolProvider,
            //
            // Provide repositories
            //
            AccountDeletionSqlDbRepository,
            AccountFetchingSqlDbRepository,
            AccountRegistrationSqlDbRepository,
            AccountUpdatingSqlDbRepository,
            AccountTagDeletionSqlDbRepository,
            AccountTagRegistrationSqlDbRepository,
            AccountTagUpdatingSqlDbRepository,
            ErrorCodeDeletionSqlDbRepository,
            ErrorCodeFetchingSqlDbRepository,
            ErrorCodeRegistrationSqlDbRepository,
            ErrorCodeUpdatingSqlDbRepository,
            GuestRoleDeletionSqlDbRepository,
            GuestRoleFetchingSqlDbRepository,
            GuestRoleRegistrationSqlDbRepository,
            GuestRoleUpdatingSqlDbRepository,
            GuestUserDeletionSqlDbRepository,
            GuestUserFetchingSqlDbRepository,
            GuestUserOnAccountUpdatingSqlDbRepository,
            GuestUserRegistrationSqlDbRepository,
            ProfileFetchingSqlDbRepository,
            LicensedResourcesFetchingSqlDbRepository,
            TenantDeletionSqlDbRepository,
            TenantFetchingSqlDbRepository,
            TenantRegistrationSqlDbRepository,
            TenantUpdatingSqlDbRepository,
            TenantTagDeletionSqlDbRepository,
            TenantTagRegistrationSqlDbRepository,
            TenantTagUpdatingSqlDbRepository,
            TokenFetchingSqlDbRepository,
            TokenInvalidationSqlDbRepository,
            TokenRegistrationSqlDbRepository,
            UserDeletionSqlDbRepository,
            UserFetchingSqlDbRepository,
            UserRegistrationSqlDbRepository,
            UserUpdatingSqlDbRepository,
            WebHookDeletionSqlDbRepository,
            WebHookFetchingSqlDbRepository,
            WebHookRegistrationSqlDbRepository,
            WebHookUpdatingSqlDbRepository,
        ],
        providers = []
    }
}
