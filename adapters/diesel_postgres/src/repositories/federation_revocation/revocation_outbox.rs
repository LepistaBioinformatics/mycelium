use super::map_outbox_row::{map_outbox_row_to_dto, map_revocation_to_row};
use crate::{
    models::{config::DbPoolProvider, federation_revocation::OutboxRow},
    schema::federation_outbox as outbox_model,
};

use async_trait::async_trait;
use chrono::{DateTime, Local, Utc};
use diesel::{
    prelude::*,
    r2d2::{ConnectionManager, PooledConnection},
};
use myc_core::domain::{
    dtos::federation::{OutboxEntry, PendingRevocation},
    entities::RevocationOutbox,
};
use mycelium_base::utils::errors::{creation_err, MappedErrors};
use shaku::Component;
use std::{sync::Arc, time::Duration};
use uuid::Uuid;

const PENDING: &str = "pending";
const FAILED: &str = "failed";

#[derive(Component)]
#[shaku(interface = RevocationOutbox)]
pub struct RevocationOutboxSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn DbPoolProvider>,
}

impl RevocationOutboxSqlDbRepository {
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
    creation_err(format!("Failed to write the revocation outbox: {e}"))
}

fn after(lease: Duration) -> DateTime<Utc> {
    Utc::now() + chrono::Duration::from_std(lease).unwrap_or_default()
}

fn to_entries(rows: Vec<OutboxRow>) -> Result<Vec<OutboxEntry>, MappedErrors> {
    rows.into_iter().map(map_outbox_row_to_dto).collect()
}

/// Lease the due pending rows nobody else holds
fn lease_due(
    conn: &mut PgConnection,
    limit: u32,
    until: DateTime<Utc>,
) -> Result<Vec<OutboxRow>, diesel::result::Error> {
    conn.transaction(|conn| {
        let rows = outbox_model::table
            .filter(outbox_model::status.eq(PENDING))
            .filter(outbox_model::next_attempt_at.le(Utc::now()))
            .order(outbox_model::next_attempt_at.asc())
            .limit(limit as i64)
            .select(OutboxRow::as_select())
            .for_update()
            .skip_locked()
            .load::<OutboxRow>(conn)?;
        let ids = rows.iter().map(|row| row.id).collect::<Vec<_>>();

        diesel::update(
            outbox_model::table.filter(outbox_model::id.eq_any(&ids)),
        )
        .set(outbox_model::next_attempt_at.eq(until))
        .execute(conn)?;

        Ok(rows)
    })
}

#[async_trait]
impl RevocationOutbox for RevocationOutboxSqlDbRepository {
    #[tracing::instrument(name = "revocation_outbox_enqueue", skip_all)]
    async fn enqueue(
        &self,
        revocations: Vec<PendingRevocation>,
    ) -> Result<u64, MappedErrors> {
        let now = Utc::now();
        let rows = revocations
            .into_iter()
            .map(|revocation| map_revocation_to_row(revocation, now))
            .collect::<Vec<_>>();

        diesel::insert_into(outbox_model::table)
            .values(rows)
            .execute(&mut *self.connection()?)
            .map(|inserted| inserted as u64)
            .map_err(write_err)
    }

    #[tracing::instrument(name = "revocation_outbox_claim_due", skip_all)]
    async fn claim_due(
        &self,
        limit: u32,
        lease: Duration,
    ) -> Result<Vec<OutboxEntry>, MappedErrors> {
        let rows = lease_due(&mut *self.connection()?, limit, after(lease))
            .map_err(write_err)?;

        to_entries(rows)
    }

    #[tracing::instrument(name = "revocation_outbox_delivered", skip_all)]
    async fn delivered(&self, id: Uuid) -> Result<(), MappedErrors> {
        diesel::delete(outbox_model::table.find(id))
            .execute(&mut *self.connection()?)
            .map(|_| ())
            .map_err(write_err)
    }

    #[tracing::instrument(name = "revocation_outbox_retry", skip_all)]
    async fn retry(
        &self,
        id: Uuid,
        next_attempt_at: DateTime<Local>,
        error: String,
    ) -> Result<(), MappedErrors> {
        self.settle(id, PENDING, next_attempt_at.with_timezone(&Utc), error)
    }

    #[tracing::instrument(name = "revocation_outbox_give_up", skip_all)]
    async fn give_up(
        &self,
        id: Uuid,
        error: String,
    ) -> Result<(), MappedErrors> {
        self.settle(id, FAILED, Utc::now(), error)
    }

    #[tracing::instrument(name = "revocation_outbox_purge_failed", skip_all)]
    async fn purge_failed(
        &self,
        before: DateTime<Local>,
    ) -> Result<u64, MappedErrors> {
        diesel::delete(
            outbox_model::table
                .filter(outbox_model::status.eq(FAILED))
                .filter(
                    outbox_model::next_attempt_at
                        .lt(before.with_timezone(&Utc)),
                ),
        )
        .execute(&mut *self.connection()?)
        .map(|deleted| deleted as u64)
        .map_err(write_err)
    }
}

impl RevocationOutboxSqlDbRepository {
    /// Count the attempt and move the row to `status` at `at`
    fn settle(
        &self,
        id: Uuid,
        status: &str,
        at: DateTime<Utc>,
        error: String,
    ) -> Result<(), MappedErrors> {
        diesel::update(outbox_model::table.find(id))
            .set((
                outbox_model::attempts.eq(outbox_model::attempts + 1),
                outbox_model::status.eq(status),
                outbox_model::next_attempt_at.eq(at),
                outbox_model::last_error.eq(Some(error)),
            ))
            .execute(&mut *self.connection()?)
            .map(|_| ())
            .map_err(write_err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::federation_link::live_link_schema::{
        live_pool, provider, user_and_peer,
    };

    use myc_core::domain::dtos::federation::{
        SubjectRef, Tombstone, TombstoneReason,
    };

    fn revocation(peer_id: Uuid) -> PendingRevocation {
        PendingRevocation {
            peer_id,
            tombstone: Tombstone {
                subject_ref: SubjectRef::mint(),
                reason: TombstoneReason::LinkRevoked,
                issued_at: Local::now(),
            },
        }
    }

    #[tokio::test]
    async fn queued_rows_are_leased_retried_given_up_and_purged() {
        let Some(pool) = live_pool() else { return };
        let outbox = RevocationOutboxSqlDbRepository {
            db_config: provider(pool.clone()),
        };
        let (_, peer) = user_and_peer(&pool);
        let lease = Duration::from_secs(60);

        let queued = outbox
            .enqueue(vec![revocation(peer), revocation(peer)])
            .await
            .unwrap();
        assert_eq!(queued, 2);

        let mine = |entries: Vec<OutboxEntry>| {
            entries
                .into_iter()
                .filter(|entry| entry.revocation.peer_id == peer)
                .collect::<Vec<_>>()
        };
        let claimed = mine(outbox.claim_due(50, lease).await.unwrap());
        assert_eq!(claimed.len(), 2);
        assert_eq!(
            claimed[0].revocation.tombstone.reason,
            TombstoneReason::LinkRevoked
        );
        assert!(mine(outbox.claim_due(50, lease).await.unwrap()).is_empty());

        let past = Local::now() - chrono::Duration::seconds(1);
        outbox
            .retry(claimed[0].id, past, "refused".into())
            .await
            .unwrap();
        let retried = mine(outbox.claim_due(50, lease).await.unwrap());
        assert_eq!(retried.len(), 1);
        assert_eq!(retried[0].attempts, 1);

        let queued = [claimed[0].id, claimed[1].id];
        outbox.delivered(queued[0]).await.unwrap();
        outbox.give_up(queued[1], "gone".into()).await.unwrap();
        let future = Local::now() + chrono::Duration::days(1);
        assert!(outbox.purge_failed(future).await.unwrap() >= 1);
    }
}
