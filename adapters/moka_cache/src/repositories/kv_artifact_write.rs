use crate::config::MokaCacheProvider;

use async_trait::async_trait;
use myc_core::domain::entities::KVArtifactWrite;
use mycelium_base::{
    entities::CreateResponseKind, utils::errors::MappedErrors,
};
use shaku::Component;
use std::{sync::Arc, time::Duration};

#[derive(Component)]
#[shaku(interface = KVArtifactWrite)]
pub struct KVArtifactWriteRepository {
    #[shaku(inject)]
    pub(crate) provider: Arc<dyn MokaCacheProvider>,
}

#[async_trait]
impl KVArtifactWrite for KVArtifactWriteRepository {
    #[tracing::instrument(name = "set_encoded_artifact", skip_all)]
    async fn set_encoded_artifact(
        &self,
        key: String,
        value: String,
        ttl: u64,
    ) -> Result<CreateResponseKind<String>, MappedErrors> {
        self.provider
            .get_cache()
            .insert(key, (value.to_owned(), Duration::from_secs(ttl)))
            .await;

        Ok(CreateResponseKind::Created(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::MokaCacheProviderImpl;
    use crate::repositories::KVArtifactReadRepository;
    use myc_core::domain::entities::KVArtifactRead;
    use mycelium_base::entities::FetchResponseKind;

    fn repos() -> (KVArtifactWriteRepository, KVArtifactReadRepository) {
        let provider: Arc<dyn MokaCacheProvider> =
            Arc::new(MokaCacheProviderImpl::new());

        (
            KVArtifactWriteRepository {
                provider: provider.clone(),
            },
            KVArtifactReadRepository { provider },
        )
    }

    #[tokio::test]
    async fn set_then_get_round_trips_through_moka() {
        let (write_repo, read_repo) = repos();

        write_repo
            .set_encoded_artifact(
                "artifact-key".to_string(),
                "artifact-value".to_string(),
                60,
            )
            .await
            .unwrap();

        let found = read_repo
            .get_encoded_artifact("artifact-key".to_string())
            .await
            .unwrap();

        assert!(
            matches!(found, FetchResponseKind::Found(v) if v == "artifact-value")
        );
    }

    #[tokio::test]
    async fn missing_key_returns_not_found() {
        let (_, read_repo) = repos();

        let found = read_repo
            .get_encoded_artifact("missing-key".to_string())
            .await
            .unwrap();

        assert!(matches!(found, FetchResponseKind::NotFound(_)));
    }

    #[tokio::test]
    async fn entry_expires_after_its_own_ttl() {
        let (write_repo, read_repo) = repos();

        write_repo
            .set_encoded_artifact(
                "short-lived".to_string(),
                "value".to_string(),
                1,
            )
            .await
            .unwrap();

        tokio::time::sleep(Duration::from_millis(1_200)).await;

        let found = read_repo
            .get_encoded_artifact("short-lived".to_string())
            .await
            .unwrap();

        assert!(matches!(found, FetchResponseKind::NotFound(_)));
    }
}
