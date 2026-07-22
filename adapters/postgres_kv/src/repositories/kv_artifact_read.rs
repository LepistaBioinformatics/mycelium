use crate::{config::PgKvPoolProvider, schema::kv_artifact};

use async_trait::async_trait;
use diesel::prelude::*;
use myc_core::domain::{
    dtos::native_error_codes::NativeErrorCodes, entities::KVArtifactRead,
};
use mycelium_base::{
    entities::FetchResponseKind,
    utils::errors::{creation_err, fetching_err, MappedErrors},
};
use shaku::Component;
use std::sync::Arc;

#[derive(Component)]
#[shaku(interface = KVArtifactRead)]
pub struct KVArtifactReadRepository {
    #[shaku(inject)]
    pub pool_provider: Arc<dyn PgKvPoolProvider>,
}

#[async_trait]
impl KVArtifactRead for KVArtifactReadRepository {
    #[tracing::instrument(name = "get_encoded_artifact", skip_all)]
    async fn get_encoded_artifact(
        &self,
        key: String,
    ) -> Result<FetchResponseKind<String, String>, MappedErrors> {
        let mut conn = self.pool_provider.get_pool().get().map_err(|e| {
            creation_err(format!("Failed to get DB connection: {e}"))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let found = kv_artifact::table
            .filter(kv_artifact::key.eq(&key))
            .filter(kv_artifact::expires_at.gt(chrono::Utc::now()))
            .select(kv_artifact::value)
            .first::<String>(&mut conn)
            .optional()
            .map_err(|e| {
                fetching_err(format!("Failed to fetch artifact: {e}"))
            })?;

        let Some(value) = found else {
            return Ok(FetchResponseKind::NotFound(Some(key)));
        };

        Ok(FetchResponseKind::Found(value))
    }
}
