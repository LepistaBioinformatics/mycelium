use crate::{config::PgKvPoolProvider, schema::kv_artifact};

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use diesel::prelude::*;
use myc_core::domain::{
    dtos::native_error_codes::NativeErrorCodes, entities::KVArtifactWrite,
};
use mycelium_base::{
    entities::CreateResponseKind,
    utils::errors::{creation_err, MappedErrors},
};
use shaku::Component;
use std::sync::Arc;

#[derive(Component)]
#[shaku(interface = KVArtifactWrite)]
pub struct KVArtifactWriteRepository {
    #[shaku(inject)]
    pub pool_provider: Arc<dyn PgKvPoolProvider>,
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
        let mut conn = self.pool_provider.get_pool().get().map_err(|e| {
            creation_err(format!("Failed to get DB connection: {e}"))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let expires_at = expires_at_from(Utc::now(), ttl);

        // NOTE: the upsert round-trip and TTL expiry behavior against a live
        // Postgres backend are covered by integration testing.
        diesel::insert_into(kv_artifact::table)
            .values((
                kv_artifact::key.eq(&key),
                kv_artifact::value.eq(&value),
                kv_artifact::expires_at.eq(expires_at),
            ))
            .on_conflict(kv_artifact::key)
            .do_update()
            .set((
                kv_artifact::value.eq(&value),
                kv_artifact::expires_at.eq(expires_at),
            ))
            .execute(&mut conn)
            .map_err(|e| {
                creation_err(format!("Failed to set artifact: {e}"))
            })?;

        Ok(CreateResponseKind::Created(value))
    }
}

fn expires_at_from(now: DateTime<Utc>, ttl: u64) -> DateTime<Utc> {
    now + Duration::seconds(ttl as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expires_at_from_adds_ttl_seconds_to_now() {
        let now = DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap();

        let expires_at = expires_at_from(now, 60);

        assert_eq!(expires_at, now + Duration::seconds(60));
        assert_eq!((expires_at - now).num_seconds(), 60);
    }

    #[test]
    fn expires_at_from_zero_ttl_equals_now() {
        let now = DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap();

        assert_eq!(expires_at_from(now, 0), now);
    }
}
