use async_trait::async_trait;
use mycelium_base::{
    entities::DeletionResponseKind, utils::errors::MappedErrors,
};
use shaku::Interface;
use uuid::Uuid;

#[async_trait]
pub trait UserDeletion: Interface + Send + Sync {
    async fn delete(
        &self,
        user_id: Uuid,
    ) -> Result<DeletionResponseKind<Uuid>, MappedErrors>;
}
