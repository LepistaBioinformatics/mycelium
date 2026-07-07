use crate::config::MokaCacheProvider;

use async_trait::async_trait;
use myc_core::domain::entities::KVArtifactRead;
use mycelium_base::{entities::FetchResponseKind, utils::errors::MappedErrors};
use shaku::Component;
use std::sync::Arc;

#[derive(Component)]
#[shaku(interface = KVArtifactRead)]
pub struct KVArtifactReadRepository {
    #[shaku(inject)]
    pub(crate) provider: Arc<dyn MokaCacheProvider>,
}

#[async_trait]
impl KVArtifactRead for KVArtifactReadRepository {
    #[tracing::instrument(name = "get_encoded_artifact", skip_all)]
    async fn get_encoded_artifact(
        &self,
        key: String,
    ) -> Result<FetchResponseKind<String, String>, MappedErrors> {
        match self.provider.get_cache().get(&key).await {
            Some((value, _ttl)) => Ok(FetchResponseKind::Found(value)),
            None => Ok(FetchResponseKind::NotFound(Some(key))),
        }
    }
}
