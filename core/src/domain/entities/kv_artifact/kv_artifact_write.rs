use async_trait::async_trait;
use mycelium_base::{entities::FetchResponseKind, utils::errors::MappedErrors};
use shaku::Interface;
use uuid::Uuid;

#[async_trait]
pub trait KVArtifactWrite: Interface + Send + Sync {
    async fn set_encoded_artifact(
        &self,
        key: String,
        value: String,
    ) -> Result<FetchResponseKind<String, Uuid>, MappedErrors>;
}
