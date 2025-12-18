use crate::domain::dtos::token::{
    PublicConnectionStringInfo, Token, UserAccountScope,
};

use async_trait::async_trait;
use mycelium_base::{
    entities::{FetchManyResponseKind, FetchResponseKind},
    utils::errors::MappedErrors,
};
use shaku::Interface;
use uuid::Uuid;

#[async_trait]
pub trait TokenFetching: Interface + Send + Sync {
    /// Get token by UserAccountWithPermissionedRolesScope scope
    ///
    /// This should be used to get connection strings filtering by scope
    /// containing user account with permissioned roles.
    ///
    async fn get_connection_string(
        &self,
        scope: UserAccountScope,
    ) -> Result<FetchResponseKind<Token, String>, MappedErrors>;

    /// List connection strings by account id
    ///
    /// This should be used to list all connection strings for a given account
    /// id, filtering by the scope containing the account id.
    ///
    async fn list_connection_strings_by_account_id(
        &self,
        account_id: Uuid,
    ) -> Result<FetchManyResponseKind<PublicConnectionStringInfo>, MappedErrors>;
}
