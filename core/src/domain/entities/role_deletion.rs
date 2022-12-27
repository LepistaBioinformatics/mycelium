use async_trait::async_trait;
use clean_base::{
    entities::default_response::DeletionResponseKind,
    utils::errors::MappedErrors,
};
use shaku::Interface;
use uuid::Uuid;

#[async_trait]
pub trait RoleDeletion: Interface + Send + Sync {
    async fn delete(
        &self,
        role_id: Uuid,
    ) -> Result<DeletionResponseKind<Uuid>, MappedErrors>;
}
