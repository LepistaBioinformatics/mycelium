use super::lock_link;
use crate::{
    models::config::DbPoolProvider,
    schema::{
        federation_assertion_cache as assertion_model,
        federation_tombstone as tombstone_model,
    },
};

use async_trait::async_trait;
use chrono::{DateTime, Local, Utc};
use diesel::{
    prelude::*,
    r2d2::{ConnectionManager, PooledConnection},
    sql_types::{Timestamptz, Uuid as SqlUuid, Varchar},
};
use myc_core::domain::{
    dtos::federation::Tombstone, entities::TombstoneRegistration,
};
use mycelium_base::utils::errors::{creation_err, MappedErrors};
use shaku::Component;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = TombstoneRegistration)]
pub struct TombstoneRegistrationSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn DbPoolProvider>,
}

impl TombstoneRegistrationSqlDbRepository {
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
    creation_err(format!("Failed to record a tombstone: {e}"))
}

/// Keep the latest tombstone of the link: a newer one replaces it, an older
/// or repeated one changes nothing
const KEEP_LATEST: &str = "
    INSERT INTO federation_tombstone (link_id, peer_id, reason, issued_at)
    VALUES ($1, $2, $3, $4)
    ON CONFLICT (link_id) DO UPDATE
    SET reason = excluded.reason,
        issued_at = excluded.issued_at,
        received_at = now()
    WHERE federation_tombstone.issued_at < excluded.issued_at";

fn keep_latest(
    conn: &mut PgConnection,
    link_id: Uuid,
    peer_id: Uuid,
    tombstone: &Tombstone,
) -> Result<(), diesel::result::Error> {
    diesel::sql_query(KEEP_LATEST)
        .bind::<SqlUuid, _>(link_id)
        .bind::<SqlUuid, _>(peer_id)
        .bind::<Varchar, _>(tombstone.reason.as_str())
        .bind::<Timestamptz, _>(tombstone.issued_at.with_timezone(&Utc))
        .execute(conn)
        .map(|_| ())
}

#[async_trait]
impl TombstoneRegistration for TombstoneRegistrationSqlDbRepository {
    #[tracing::instrument(name = "tombstone_record", skip_all)]
    async fn record(
        &self,
        link_id: Uuid,
        peer_id: Uuid,
        tombstone: &Tombstone,
    ) -> Result<(), MappedErrors> {
        self.connection()?
            .transaction(|conn| {
                lock_link(conn, link_id)?;
                keep_latest(conn, link_id, peer_id, tombstone)?;
                diesel::delete(
                    assertion_model::table
                        .filter(assertion_model::link_id.eq(link_id)),
                )
                .execute(conn)
                .map(|_| ())
            })
            .map_err(write_err)
    }

    #[tracing::instrument(name = "tombstone_purge_older_than", skip_all)]
    async fn purge_older_than(
        &self,
        before: DateTime<Local>,
    ) -> Result<u64, MappedErrors> {
        diesel::delete(tombstone_model::table.filter(
            tombstone_model::received_at.lt(before.with_timezone(&Utc)),
        ))
        .execute(&mut *self.connection()?)
        .map(|deleted| deleted as u64)
        .map_err(write_err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::{
        federation_assertion::AssertionStoreWriteSqlDbRepository,
        federation_link::live_link_schema::{active_link, live_pool, provider},
    };

    use chrono::{Duration, Timelike};
    use myc_core::domain::{
        dtos::federation::{
            ProfileAssertion, StoredAssertion, SubjectRef, TombstoneReason,
            VerifiedAssertion,
        },
        entities::AssertionStoreWrite,
    };

    fn issued(
        link_id: Uuid,
        peer_id: Uuid,
        at: DateTime<Local>,
    ) -> StoredAssertion {
        StoredAssertion {
            link_id,
            peer_id,
            assertion: VerifiedAssertion {
                jws: "jws".to_string(),
                payload: ProfileAssertion::default(),
                issued_at: at,
                expires_at: at + Duration::seconds(60),
            },
        }
    }

    fn tombstone(at: DateTime<Local>) -> Tombstone {
        Tombstone {
            subject_ref: SubjectRef::mint(),
            reason: TombstoneReason::GrantsChanged,
            issued_at: at,
        }
    }

    #[tokio::test]
    async fn a_tombstone_forgets_and_refuses_older_assertions_and_keeps_the_latest(
    ) {
        let Some(pool) = live_pool() else { return };
        let provider = provider(pool.clone());
        let tombstones = TombstoneRegistrationSqlDbRepository {
            db_config: provider.clone(),
        };
        let store = AssertionStoreWriteSqlDbRepository {
            db_config: provider,
        };
        let (link, peer) = active_link(&pool);
        // Whole seconds, as a JWT `iat`; Postgres keeps microseconds only
        let now = Local::now().with_nanosecond(0).unwrap();

        assert!(store.store(issued(link, peer, now)).await.unwrap());
        tombstones
            .record(link, peer, &tombstone(now))
            .await
            .unwrap();
        let older = now - Duration::seconds(30);
        tombstones
            .record(link, peer, &tombstone(older))
            .await
            .unwrap();

        assert!(!store.store(issued(link, peer, now)).await.unwrap());
        let later = now + Duration::seconds(1);
        assert!(store.store(issued(link, peer, later)).await.unwrap());
        assert_eq!(store.forget_peer(peer).await.unwrap(), 1);
        let future = now + Duration::days(1);
        assert!(tombstones.purge_older_than(future).await.unwrap() >= 1);
    }
}
