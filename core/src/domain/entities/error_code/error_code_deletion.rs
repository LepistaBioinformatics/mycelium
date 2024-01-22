use async_trait::async_trait;
use mycelium_base::{
    entities::DeletionResponseKind, utils::errors::MappedErrors,
};
use shaku::Interface;

#[async_trait]
pub trait ErrorCodeDeletion: Interface + Send + Sync {
    async fn delete(
        &self,
        prefix: String,
        code: i32,
    ) -> Result<DeletionResponseKind<(String, i32)>, MappedErrors>;
}
