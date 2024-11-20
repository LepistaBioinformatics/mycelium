use crate::domain::dtos::token::{
    AccountWithPermissionedRolesScope, RoleWithPermissionsScope, Token,
};

use async_trait::async_trait;
use mycelium_base::{entities::FetchResponseKind, utils::errors::MappedErrors};
use shaku::Interface;

#[async_trait]
pub trait TokenFetching: Interface + Send + Sync {
    /// Get token by AccountWithPermissionedRolesScope scope
    ///
    /// This should be used to get connection strings filtering by scope
    /// containing account with permissioned roles.
    async fn get_connection_string_by_account_with_permissioned_roles_scope(
        &self,
        scope: AccountWithPermissionedRolesScope,
    ) -> Result<FetchResponseKind<Token, String>, MappedErrors>;

    /// Get token by RoleWithPermissionsScope scope
    ///
    /// This should be used to get connection strings filtering by scope
    /// containing role with permissions.
    async fn get_connection_string_by_role_with_permissioned_roles_scope(
        &self,
        scope: RoleWithPermissionsScope,
    ) -> Result<FetchResponseKind<Token, String>, MappedErrors>;
}
