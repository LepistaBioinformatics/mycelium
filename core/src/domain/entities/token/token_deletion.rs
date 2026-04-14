use async_trait::async_trait;
use mycelium_base::{
    entities::DeletionResponseKind, utils::errors::MappedErrors,
};
use shaku::Interface;
use uuid::Uuid;

#[async_trait]
pub trait TokenDeletion: Interface + Send + Sync {
    /// Revoke a connection string by setting its expiration to now
    async fn revoke_connection_string(
        &self,
        account_id: Uuid,
        token_id: u32,
    ) -> Result<DeletionResponseKind<u32>, MappedErrors>;

    /// Hard-delete a connection string row
    async fn delete_connection_string(
        &self,
        account_id: Uuid,
        token_id: u32,
    ) -> Result<DeletionResponseKind<u32>, MappedErrors>;
}
