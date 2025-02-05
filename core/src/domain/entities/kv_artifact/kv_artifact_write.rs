use async_trait::async_trait;
use mycelium_base::{
    entities::CreateResponseKind, utils::errors::MappedErrors,
};
use shaku::Interface;

#[async_trait]
pub trait KVArtifactWrite: Interface + Send + Sync {
    async fn set_encoded_artifact(
        &self,
        key: String,
        value: String,
        ttl: u64,
    ) -> Result<CreateResponseKind<String>, MappedErrors>;
}
