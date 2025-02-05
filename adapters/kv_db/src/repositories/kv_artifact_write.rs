use async_trait::async_trait;
use myc_adapters_shared_lib::models::SharedClientProvider;
use myc_core::domain::entities::KVArtifactWrite;
use mycelium_base::{entities::FetchResponseKind, utils::errors::MappedErrors};
use shaku::Component;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = KVArtifactWrite)]
pub struct KVArtifactWriteRepository {
    #[shaku(inject)]
    client: Arc<dyn SharedClientProvider>,
}

#[async_trait]
impl KVArtifactWrite for KVArtifactWriteRepository {
    //#[tracing::instrument(name = "set", skip_all)]
    async fn set_encoded_artifact(
        &self,
        key: String,
        value: String,
    ) -> Result<FetchResponseKind<String, Uuid>, MappedErrors> {
        unimplemented!()
    }
}
