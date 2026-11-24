use super::{
    encode_message_for_queue::encode_messages_for_queue,
    register_within_quota::{register_within_quota, NotificationBatch},
};
use crate::{
    models::config::DbPoolProvider,
    schema::notification_dispatch as notification_dispatch_model,
};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use diesel::{
    prelude::*,
    r2d2::{ConnectionManager, PooledConnection},
    PgConnection,
};
use myc_core::domain::{
    dtos::{
        message::MessageSendingEvent,
        native_error_codes::NativeErrorCodes,
        notification_dispatch::{
            pair_dispatches_with_messages, NewNotificationDispatch,
            QuotaOutcome,
        },
    },
    entities::NotificationDispatchRegistration,
};
use mycelium_base::utils::errors::{creation_err, deletion_err, MappedErrors};
use shaku::Component;
use std::sync::Arc;

#[derive(Component)]
#[shaku(interface = NotificationDispatchRegistration)]
pub struct NotificationDispatchRegistrationSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn DbPoolProvider>,
}

impl NotificationDispatchRegistrationSqlDbRepository {
    fn pooled_connection(
        &self,
    ) -> Result<PooledConnection<ConnectionManager<PgConnection>>, MappedErrors>
    {
        self.db_config.get_pool().get().map_err(|e| {
            creation_err(format!("Failed to get DB connection: {e}"))
                .with_code(NativeErrorCodes::MYC00001)
        })
    }
}

#[async_trait]
impl NotificationDispatchRegistration
    for NotificationDispatchRegistrationSqlDbRepository
{
    #[tracing::instrument(
        name = "register_and_enqueue_notifications",
        skip_all
    )]
    async fn register_and_enqueue(
        &self,
        sender_tag: &str,
        quota_per_hour: u32,
        rows: Vec<NewNotificationDispatch>,
        messages: Vec<MessageSendingEvent>,
    ) -> Result<QuotaOutcome, MappedErrors> {
        pair_dispatches_with_messages(&rows, &messages)?;

        let queued_messages = encode_messages_for_queue(&messages)?;

        let batch = NotificationBatch {
            sender_tag,
            quota_per_hour,
            dispatches: &rows,
            queued_messages: &queued_messages,
        };

        let conn = &mut self.pooled_connection()?;

        conn.transaction(|conn| register_within_quota(conn, &batch))
            .map_err(|e| {
                creation_err(format!(
                    "Failed to register notification dispatch: {e}"
                ))
            })
    }

    #[tracing::instrument(name = "purge_notification_dispatches", skip_all)]
    async fn purge_older_than(
        &self,
        cutoff: DateTime<Utc>,
    ) -> Result<u64, MappedErrors> {
        let conn = &mut self.pooled_connection()?;

        let purged = diesel::delete(
            notification_dispatch_model::table
                .filter(notification_dispatch_model::created_at.lt(cutoff)),
        )
        .execute(conn)
        .map_err(|e| {
            deletion_err(format!(
                "Failed to purge notification dispatches: {e}"
            ))
        })?;

        Ok(u64::try_from(purged).unwrap_or(u64::MAX))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        models::config::DbPool, repositories::LocalMessageReadSqlDbRepository,
        schema::message_queue as message_queue_model,
    };

    use chrono::Duration;
    use diesel::{
        connection::SimpleConnection,
        r2d2::{ConnectionManager, Pool},
        PgConnection,
    };
    use myc_core::domain::{
        dtos::{
            email::Email,
            message::{FromEmail, Message, MessageStatus},
            notification_dispatch::NotificationDispatchIssuer,
        },
        entities::LocalMessageReading,
    };
    use mycelium_base::entities::FetchManyResponseKind;
    use std::sync::{Barrier, Once};
    use uuid::Uuid;

    /// Same variable as the other live tests of this crate. Without it every
    /// test below reports the skip and passes, so `cargo test --workspace`
    /// stays green on a machine with no Postgres.
    const DATABASE_URL_VAR: &str = "MYC_TEST_DATABASE_URL";

    /// Own schema, so these tests never race the other live tests.
    const SCHEMA: &str = "myc_notification_dispatch_test";

    /// The table exactly as the migration creates it (the GRANT needs psql
    /// variables and is irrelevant here).
    const MIGRATION: &str = include_str!(
        "../../../sql/migrations/20261120_01_notification_dispatch.sql"
    );

    /// `message_queue` as `up.sql` creates it, plus one test-only constraint:
    /// the real table accepts any row the repository can build, so a marked
    /// `error` is the only way to make its insert fail on demand.
    const MESSAGE_QUEUE: &str = "
        CREATE TABLE message_queue (
            id UUID DEFAULT gen_random_uuid(),
            message TEXT NOT NULL,
            created TIMESTAMPTZ DEFAULT now(),
            attempted TIMESTAMPTZ DEFAULT NULL,
            status VARCHAR(100) NOT NULL,
            attempts INT DEFAULT 0,
            error TEXT DEFAULT NULL,
            CONSTRAINT refuse_marked_message
                CHECK (error IS DISTINCT FROM 'refuse-this-message')
        );";

    static PREPARE: Once = Once::new();

    struct TestPool(DbPool);

    impl DbPoolProvider for TestPool {
        fn get_pool(&self) -> DbPool {
            self.0.clone()
        }
    }

    fn pool() -> Option<DbPool> {
        let url = std::env::var(DATABASE_URL_VAR).ok()?;
        let separator = if url.contains('?') { '&' } else { '?' };
        let url = format!("{url}{separator}options=-csearch_path%3D{SCHEMA}");

        let pool = Pool::builder()
            .max_size(8)
            .build(ConnectionManager::<PgConnection>::new(url))
            .expect("failed to build the test pool");

        PREPARE.call_once(|| prepare(&pool));

        Some(pool)
    }

    fn prepare(pool: &DbPool) {
        let conn = &mut pool.get().expect("failed to take a test connection");
        let table = MIGRATION
            .split("GRANT ALL")
            .next()
            .expect("migration has a table");

        conn.batch_execute(&format!(
            "CREATE SCHEMA IF NOT EXISTS {SCHEMA};
             DROP TABLE IF EXISTS notification_dispatch;
             DROP TABLE IF EXISTS message_queue;
             {MESSAGE_QUEUE}
             {table}"
        ))
        .expect("failed to prepare the test schema");
    }

    /// The repository over a live pool, or `None` (after reporting the skip)
    /// when no database was given.
    fn live_repository(
    ) -> Option<(NotificationDispatchRegistrationSqlDbRepository, DbPool)> {
        let Some(pool) = pool() else {
            eprintln!(
                "skipping: set {DATABASE_URL_VAR} to run the live \
                 notification dispatch tests"
            );
            return None;
        };

        let repository = NotificationDispatchRegistrationSqlDbRepository {
            db_config: Arc::new(TestPool(pool.clone())),
        };

        Some((repository, pool))
    }

    /// A sender tag no other test (nor a previous run) uses.
    fn unique_tag() -> String {
        format!("service-01-{}", &Uuid::new_v4().simple().to_string()[..12])
    }

    type Batch = (Vec<NewNotificationDispatch>, Vec<MessageSendingEvent>);

    fn batch(size: usize) -> Batch {
        (0..size)
            .map(|index| {
                let recipient =
                    Email::from_string(format!("user-{index:02}@example.com"))
                        .unwrap();
                let message = MessageSendingEvent::new(Message {
                    from: FromEmail::NamedEmail(
                        "Service 01 <noreply@example.com>".to_string(),
                    ),
                    to: recipient.clone(),
                    cc: None,
                    subject: "Invoice ready".to_string(),
                    body: "<p>Hello</p>".to_string(),
                    headers: vec![],
                });
                let dispatch = NewNotificationDispatch {
                    id: message.id,
                    issuer: NotificationDispatchIssuer {
                        connection_string_id: 7,
                        account_id: Uuid::new_v4(),
                        tenant_id: Some(Uuid::new_v4()),
                    },
                    recipient,
                    template: "invoice-ready".to_string(),
                    locale: "en-us".to_string(),
                };

                (dispatch, message)
            })
            .unzip()
    }

    async fn send(
        repository: &NotificationDispatchRegistrationSqlDbRepository,
        tag: &str,
        quota_per_hour: u32,
        size: usize,
    ) -> Result<QuotaOutcome, MappedErrors> {
        let (rows, messages) = batch(size);

        repository
            .register_and_enqueue(tag, quota_per_hour, rows, messages)
            .await
    }

    fn dispatched(pool: &DbPool, tag: &str) -> i64 {
        notification_dispatch_model::table
            .filter(notification_dispatch_model::sender_tag.eq(tag))
            .count()
            .get_result(&mut pool.get().unwrap())
            .unwrap()
    }

    fn queued(pool: &DbPool, ids: &[Uuid]) -> i64 {
        message_queue_model::table
            .filter(message_queue_model::id.eq_any(ids))
            .count()
            .get_result(&mut pool.get().unwrap())
            .unwrap()
    }

    fn age_dispatches(pool: &DbPool, tag: &str, age: Duration) {
        diesel::update(
            notification_dispatch_model::table
                .filter(notification_dispatch_model::sender_tag.eq(tag)),
        )
        .set(notification_dispatch_model::created_at.eq(Utc::now() - age))
        .execute(&mut pool.get().unwrap())
        .unwrap();
    }

    fn accepted_ids(outcome: QuotaOutcome) -> Vec<Uuid> {
        match outcome {
            QuotaOutcome::Accepted(ids) => ids,
            QuotaOutcome::Exceeded { .. } => panic!("expected Accepted"),
        }
    }

    fn retry_after(outcome: QuotaOutcome) -> u64 {
        match outcome {
            QuotaOutcome::Exceeded { retry_after_secs } => retry_after_secs,
            QuotaOutcome::Accepted(_) => panic!("expected Exceeded"),
        }
    }

    #[tokio::test]
    async fn live_batch_under_quota_is_traced_and_queued(
    ) -> Result<(), MappedErrors> {
        let Some((repository, pool)) = live_repository() else {
            return Ok(());
        };
        let tag = unique_tag();
        let (rows, messages) = batch(2);
        let expected_ids: Vec<Uuid> = rows.iter().map(|row| row.id).collect();
        let issuer = rows[0].issuer.clone();

        let outcome = repository
            .register_and_enqueue(&tag, 5, rows, messages)
            .await?;

        assert_eq!(accepted_ids(outcome), expected_ids);
        assert_eq!(dispatched(&pool, &tag), 2);
        assert_eq!(queued(&pool, &expected_ids), 2);

        let stored = notification_dispatch_model::table
            .find(expected_ids[0])
            .select((
                notification_dispatch_model::channel,
                notification_dispatch_model::connection_string_id,
                notification_dispatch_model::account_id,
                notification_dispatch_model::tenant_id,
                notification_dispatch_model::recipient,
                notification_dispatch_model::template,
                notification_dispatch_model::locale,
            ))
            .get_result::<(
                String,
                i32,
                Uuid,
                Option<Uuid>,
                String,
                String,
                String,
            )>(&mut pool.get().unwrap())
            .unwrap();

        assert_eq!(
            stored,
            (
                "email".to_string(),
                7,
                issuer.account_id,
                issuer.tenant_id,
                "user-00@example.com".to_string(),
                "invoice-ready".to_string(),
                "en-us".to_string(),
            )
        );

        Ok(())
    }

    #[tokio::test]
    async fn live_queued_message_is_readable_by_the_email_dispatcher(
    ) -> Result<(), MappedErrors> {
        let Some((repository, pool)) = live_repository() else {
            return Ok(());
        };
        let ids = accepted_ids(send(&repository, &unique_tag(), 5, 1).await?);

        let reader = LocalMessageReadSqlDbRepository {
            db_config: Arc::new(TestPool(pool)),
            visibility_timeout_secs: 240,
        };

        let FetchManyResponseKind::Found(claimed) = reader
            .list_oldest_messages(1000, MessageStatus::Queued)
            .await?
        else {
            panic!("expected queued messages");
        };

        let message = claimed
            .iter()
            .find(|event| event.id == ids[0])
            .expect("the enqueued message is claimable");

        assert_eq!(message.message.subject, "Invoice ready");
        assert_eq!(message.message.to.email(), "user-00@example.com");

        Ok(())
    }

    #[tokio::test]
    async fn live_quota_boundary_accepts_at_quota_and_rejects_one_over(
    ) -> Result<(), MappedErrors> {
        let Some((repository, pool)) = live_repository() else {
            return Ok(());
        };
        let tag = unique_tag();

        accepted_ids(send(&repository, &tag, 3, 2).await?);

        // 2 + 2 would cross the quota: the batch is refused as a whole.
        retry_after(send(&repository, &tag, 3, 2).await?);
        assert_eq!(dispatched(&pool, &tag), 2);

        // 2 + 1 lands exactly on the quota.
        accepted_ids(send(&repository, &tag, 3, 1).await?);
        assert_eq!(dispatched(&pool, &tag), 3);

        // One over.
        let (rows, messages) = batch(1);
        let refused_ids: Vec<Uuid> = rows.iter().map(|row| row.id).collect();
        let outcome = repository
            .register_and_enqueue(&tag, 3, rows, messages)
            .await?;

        retry_after(outcome);
        assert_eq!(dispatched(&pool, &tag), 3);
        assert_eq!(queued(&pool, &refused_ids), 0);

        // Another sender is not affected.
        accepted_ids(send(&repository, &unique_tag(), 3, 3).await?);

        Ok(())
    }

    #[tokio::test]
    async fn live_retry_after_follows_the_oldest_dispatch_in_the_window(
    ) -> Result<(), MappedErrors> {
        let Some((repository, pool)) = live_repository() else {
            return Ok(());
        };
        let tag = unique_tag();

        accepted_ids(send(&repository, &tag, 1, 1).await?);

        let fresh = retry_after(send(&repository, &tag, 1, 1).await?);
        assert!((3590..=3600).contains(&fresh), "{fresh}");

        age_dispatches(&pool, &tag, Duration::minutes(59));

        let nearly_out = retry_after(send(&repository, &tag, 1, 1).await?);
        assert!((50..=60).contains(&nearly_out), "{nearly_out}");

        // Once the oldest dispatch leaves the window the quota is free again.
        age_dispatches(&pool, &tag, Duration::minutes(61));

        accepted_ids(send(&repository, &tag, 1, 1).await?);

        Ok(())
    }

    #[tokio::test]
    async fn live_batch_larger_than_the_quota_never_reports_zero(
    ) -> Result<(), MappedErrors> {
        let Some((repository, pool)) = live_repository() else {
            return Ok(());
        };
        let tag = unique_tag();

        assert_eq!(retry_after(send(&repository, &tag, 2, 3).await?), 3600);
        assert_eq!(dispatched(&pool, &tag), 0);

        Ok(())
    }

    /// DEN-17: the quota holds across pods. Every thread takes its own
    /// connection and they all start together, so without the per-sender
    /// lock several of them would count the same window and overshoot.
    #[test]
    fn live_concurrent_sends_never_exceed_the_quota() {
        const THREADS: usize = 8;
        const SENDS_PER_THREAD: usize = 5;
        const QUOTA: u32 = 7;

        let Some((repository, pool)) = live_repository() else {
            return;
        };
        let tag = unique_tag();
        let barrier = Barrier::new(THREADS);

        let accepted: usize = std::thread::scope(|scope| {
            let workers: Vec<_> = (0..THREADS)
                .map(|_| {
                    scope.spawn(|| {
                        barrier.wait();

                        (0..SENDS_PER_THREAD)
                            .filter(|_| {
                                let outcome = futures::executor::block_on(
                                    send(&repository, &tag, QUOTA, 1),
                                )
                                .unwrap();

                                matches!(outcome, QuotaOutcome::Accepted(_))
                            })
                            .count()
                    })
                })
                .collect();

            workers
                .into_iter()
                .map(|worker| worker.join().unwrap())
                .sum()
        });

        assert_eq!(accepted, QUOTA as usize);
        assert_eq!(dispatched(&pool, &tag), i64::from(QUOTA));
    }

    #[tokio::test]
    async fn live_failed_enqueue_rolls_the_dispatch_rows_back(
    ) -> Result<(), MappedErrors> {
        let Some((repository, pool)) = live_repository() else {
            return Ok(());
        };
        let tag = unique_tag();
        let (rows, mut messages) = batch(2);
        let ids: Vec<Uuid> = rows.iter().map(|row| row.id).collect();

        // See `MESSAGE_QUEUE`: the second message is refused by the queue.
        messages[1].error = Some("refuse-this-message".to_string());

        let result = repository
            .register_and_enqueue(&tag, 5, rows, messages)
            .await;

        assert!(result.is_err());
        assert_eq!(dispatched(&pool, &tag), 0);
        assert_eq!(queued(&pool, &ids), 0);

        // The failed attempt consumed no quota and left no lock behind.
        accepted_ids(send(&repository, &tag, 2, 2).await?);

        Ok(())
    }

    #[tokio::test]
    async fn live_unpaired_batch_writes_nothing() -> Result<(), MappedErrors> {
        let Some((repository, pool)) = live_repository() else {
            return Ok(());
        };
        let tag = unique_tag();
        let (rows, _) = batch(1);
        let (_, other_messages) = batch(1);

        let result = repository
            .register_and_enqueue(&tag, 5, rows, other_messages)
            .await;

        assert!(result.is_err());
        assert_eq!(dispatched(&pool, &tag), 0);

        Ok(())
    }

    #[tokio::test]
    async fn live_sender_tag_is_never_interpreted_as_sql(
    ) -> Result<(), MappedErrors> {
        let Some((repository, pool)) = live_repository() else {
            return Ok(());
        };
        let tag = "x'||pg_sleep(30)||'; --";

        accepted_ids(send(&repository, tag, 1, 1).await?);
        retry_after(send(&repository, tag, 1, 1).await?);

        assert_eq!(dispatched(&pool, tag), 1);

        Ok(())
    }

    #[tokio::test]
    async fn live_purge_deletes_only_rows_older_than_the_cutoff(
    ) -> Result<(), MappedErrors> {
        let Some((repository, pool)) = live_repository() else {
            return Ok(());
        };
        let expired_tag = unique_tag();
        let kept_tag = unique_tag();

        accepted_ids(send(&repository, &expired_tag, 5, 2).await?);
        accepted_ids(send(&repository, &kept_tag, 5, 1).await?);
        age_dispatches(&pool, &expired_tag, Duration::days(91));
        age_dispatches(&pool, &kept_tag, Duration::days(89));

        let purged = repository
            .purge_older_than(Utc::now() - Duration::days(90))
            .await?;

        assert_eq!(purged, 2);
        assert_eq!(dispatched(&pool, &expired_tag), 0);
        assert_eq!(dispatched(&pool, &kept_tag), 1);

        Ok(())
    }
}
