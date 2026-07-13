pub(crate) mod internal_error;
mod optional_written_by_parser;

pub mod account;
pub mod account_tag;
pub mod encryption_key;
pub mod error_code;
pub mod guest_role;
pub mod guest_user;
pub mod instance_settings;
pub mod licensed_resources;
pub mod message;
pub mod profile;
pub mod tenant;
pub mod tenant_tag;
pub mod token;
pub mod user;
pub mod webhook;

use shaku::module;

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

use crate::config::DieselSqliteDbPoolProvider;

module! {
    pub SqlAppModule {
        components = [
            //
            // Provide the database pool
            //
            DieselSqliteDbPoolProvider,
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
