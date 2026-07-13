use shaku::module;

mod account;
mod account_tag;
mod config;
mod encryption_key;
mod error_code;
mod guest_role;
mod guest_user;
mod instance_settings;
mod licensed_resources;
mod message;
mod optional_written_by_parser;
mod profile;
mod resource_audit_log;
mod tenant;
mod tenant_tag;
mod token;
mod user;
mod webhook;

use account::*;
use account_tag::*;
use encryption_key::*;
use error_code::*;
use guest_role::*;
use guest_user::*;
use instance_settings::*;
use licensed_resources::*;
use message::*;
use optional_written_by_parser::*;
use profile::*;
use tenant::*;
use tenant_tag::*;
use token::*;
use user::*;
use webhook::*;

pub use config::*;
pub use resource_audit_log::*;

module! {
    pub SqlAppModule {
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
            EncryptionKeyFetchingSqlDbRepository,
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
            GuestUserOnAccountFetchingSqlDbRepository,
            GuestUserRegistrationSqlDbRepository,
            InstanceSettingsFetchingSqlDbRepository,
            InstanceSettingsRegistrationSqlDbRepository,
            ProfileFetchingSqlDbRepository,
            LicensedResourcesFetchingSqlDbRepository,
            LocalMessageReadSqlDbRepository,
            LocalMessageWriteSqlDbRepository,
            ResourceAuditLogFetchingSqlDbRepository,
            ResourceAuditLogRegistrationSqlDbRepository,
            TenantDeletionSqlDbRepository,
            TenantFetchingSqlDbRepository,
            TenantRegistrationSqlDbRepository,
            TenantUpdatingSqlDbRepository,
            TenantTagDeletionSqlDbRepository,
            TenantTagRegistrationSqlDbRepository,
            TenantTagUpdatingSqlDbRepository,
            TokenDeletionSqlDbRepository,
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
