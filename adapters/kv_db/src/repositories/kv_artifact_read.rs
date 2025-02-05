use async_trait::async_trait;
use myc_adapters_shared_lib::models::SharedClientProvider;
use myc_core::domain::entities::KVArtifactRead;
use mycelium_base::{entities::FetchResponseKind, utils::errors::MappedErrors};
use shaku::Component;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = KVArtifactRead)]
pub struct KVArtifactReadRepository {
    #[shaku(inject)]
    client: Arc<dyn SharedClientProvider>,
}

#[async_trait]
impl KVArtifactRead for KVArtifactReadRepository {
    //#[tracing::instrument(name = "set", skip_all)]
    async fn get_encoded_artifact(
        &self,
        key: String,
    ) -> Result<FetchResponseKind<String, Uuid>, MappedErrors> {
        unimplemented!()
    }
}
