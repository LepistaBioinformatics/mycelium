use async_trait::async_trait;
use mycelium_base::{
    entities::DeletionResponseKind, utils::errors::MappedErrors,
};
use shaku::Interface;

#[async_trait]
pub trait SessionTokenDeletion: Interface + Send + Sync {
    async fn delete(
        &self,
        session_key: String,
    ) -> Result<DeletionResponseKind<String>, MappedErrors>;
}
