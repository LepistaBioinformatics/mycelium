use super::map_assertion_row::map_assertion_dto_to_row;
use crate::{
    models::{config::DbPoolProvider, federation_assertion::AssertionRow},
    repositories::federation_revocation::lock_link,
    schema::{
        federation_assertion_cache as assertion_model,
        federation_tombstone as tombstone_model,
    },
};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use diesel::{
    prelude::*,
    r2d2::{ConnectionManager, PooledConnection},
    upsert::excluded,
};
use myc_core::domain::{
    dtos::federation::StoredAssertion, entities::AssertionStoreWrite,
};
use mycelium_base::utils::errors::{creation_err, MappedErrors};
use shaku::Component;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = AssertionStoreWrite)]
pub struct AssertionStoreWriteSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn DbPoolProvider>,
}

impl AssertionStoreWriteSqlDbRepository {
    fn connection(
        &self,
    ) -> Result<PooledConnection<ConnectionManager<PgConnection>>, MappedErrors>
    {
        self.db_config.get_pool().get().map_err(|e| {
            creation_err(format!("Failed to get DB connection: {e}"))
        })
    }
}

fn write_err(e: diesel::result::Error) -> MappedErrors {
    creation_err(format!("Failed to write the assertion store: {e}"))
}

/// Whether the link's tombstone was issued no earlier than the assertion
fn superseded(
    conn: &mut PgConnection,
    row: &AssertionRow,
) -> Result<bool, diesel::result::Error> {
    let revoked_at = tombstone_model::table
        .find(row.link_id)
        .select(tombstone_model::issued_at)
        .first::<DateTime<Utc>>(conn)
        .optional()?;

    Ok(revoked_at.is_some_and(|revoked_at| row.iat <= revoked_at))
}

/// Keep the assertion, replacing the link's previous one
fn upsert(
    conn: &mut PgConnection,
    row: AssertionRow,
) -> Result<usize, diesel::result::Error> {
    diesel::insert_into(assertion_model::table)
        .values(row)
        .on_conflict((assertion_model::link_id, assertion_model::kind))
        .do_update()
        .set((
            assertion_model::peer_id.eq(excluded(assertion_model::peer_id)),
            assertion_model::jws.eq(excluded(assertion_model::jws)),
            assertion_model::payload.eq(excluded(assertion_model::payload)),
            assertion_model::iat.eq(excluded(assertion_model::iat)),
            assertion_model::exp.eq(excluded(assertion_model::exp)),
            assertion_model::received_at.eq(Utc::now()),
        ))
        .execute(conn)
}

#[async_trait]
impl AssertionStoreWrite for AssertionStoreWriteSqlDbRepository {
    #[tracing::instrument(name = "assertion_store_store", skip_all)]
    async fn store(
        &self,
        assertion: StoredAssertion,
    ) -> Result<bool, MappedErrors> {
        let row = map_assertion_dto_to_row(assertion)?;

        self.connection()?
            .transaction(|conn| {
                lock_link(conn, row.link_id)?;

                if superseded(conn, &row)? {
                    return Ok(false);
                }

                upsert(conn, row).map(|_| true)
            })
            .map_err(write_err)
    }

    #[tracing::instrument(name = "assertion_store_forget", skip_all)]
    async fn forget(&self, link_id: Uuid) -> Result<(), MappedErrors> {
        diesel::delete(
            assertion_model::table.filter(assertion_model::link_id.eq(link_id)),
        )
        .execute(&mut *self.connection()?)
        .map(|_| ())
        .map_err(write_err)
    }

    #[tracing::instrument(name = "assertion_store_forget_peer", skip_all)]
    async fn forget_peer(&self, peer_id: Uuid) -> Result<u64, MappedErrors> {
        diesel::delete(
            assertion_model::table.filter(assertion_model::peer_id.eq(peer_id)),
        )
        .execute(&mut *self.connection()?)
        .map(|deleted| deleted as u64)
        .map_err(write_err)
    }

    #[tracing::instrument(name = "assertion_store_forget_all", skip_all)]
    async fn forget_all(&self) -> Result<u64, MappedErrors> {
        diesel::delete(assertion_model::table)
            .execute(&mut *self.connection()?)
            .map(|deleted| deleted as u64)
            .map_err(write_err)
    }

    #[tracing::instrument(name = "assertion_store_purge_expired", skip_all)]
    async fn purge_expired(&self) -> Result<u64, MappedErrors> {
        diesel::delete(
            assertion_model::table.filter(assertion_model::exp.le(Utc::now())),
        )
        .execute(&mut *self.connection()?)
        .map(|deleted| deleted as u64)
        .map_err(write_err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::{
        federation_assertion::AssertionStoreReadSqlDbRepository,
        federation_link::live_link_schema::{active_link, live_pool, provider},
    };

    use chrono::{Duration, Local};
    use myc_core::domain::{
        dtos::federation::{ProfileAssertion, VerifiedAssertion},
        entities::AssertionStoreRead,
    };

    fn stored(
        link_id: Uuid,
        peer_id: Uuid,
        jws: &str,
        lifetime: Duration,
    ) -> StoredAssertion {
        let now = Local::now();
        StoredAssertion {
            link_id,
            peer_id,
            assertion: VerifiedAssertion {
                jws: jws.to_string(),
                payload: ProfileAssertion::default(),
                issued_at: now,
                expires_at: now + lifetime,
            },
        }
    }

    #[tokio::test]
    async fn storing_again_replaces_and_expired_rows_are_neither_served_nor_kept(
    ) {
        let Some(pool) = live_pool() else { return };
        let provider = provider(pool.clone());
        let write = AssertionStoreWriteSqlDbRepository {
            db_config: provider.clone(),
        };
        let read = AssertionStoreReadSqlDbRepository {
            db_config: provider,
        };
        let (fresh, peer) = active_link(&pool);
        let (stale, stale_peer) = active_link(&pool);

        let minute = Duration::seconds(60);
        write
            .store(stored(fresh, peer, "first", minute))
            .await
            .unwrap();
        write
            .store(stored(fresh, peer, "second", minute))
            .await
            .unwrap();
        let expired = stored(stale, stale_peer, "old", -minute);
        write.store(expired).await.unwrap();

        let valid = read.list_valid_for_links(&[fresh, stale]).await.unwrap();
        assert_eq!(valid.len(), 1);
        assert_eq!(valid[0].assertion.jws, "second");
        assert!(read.list_valid(Some(stale_peer)).await.unwrap().is_empty());
        assert!(write.purge_expired().await.unwrap() >= 1);

        write.forget(fresh).await.unwrap();
        assert!(read
            .list_valid_for_links(&[fresh])
            .await
            .unwrap()
            .is_empty());
    }
}
