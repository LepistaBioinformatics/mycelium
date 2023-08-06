use async_trait::async_trait;
use clean_base::{entities::DeletionResponseKind, utils::errors::MappedErrors};
use shaku::Interface;
use uuid::Uuid;

#[async_trait]
pub trait WebHookDeletion: Interface + Send + Sync {
    async fn delete(
        &self,
        hook_id: Uuid,
    ) -> Result<DeletionResponseKind<Uuid>, MappedErrors>;
}
