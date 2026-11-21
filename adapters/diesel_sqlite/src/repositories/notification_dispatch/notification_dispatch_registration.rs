use super::{
    dispatch_timestamp_to_text::dispatch_timestamp_to_text,
    encode_message_for_queue::encode_messages_for_queue,
    register_within_quota::{register_within_quota, NotificationBatch},
};
use crate::{config::SqliteDbPoolProvider, schema::notification_dispatch};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use diesel::{
    prelude::*,
    r2d2::{ConnectionManager, PooledConnection},
    SqliteConnection,
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
    pub db_config: Arc<dyn SqliteDbPoolProvider>,
}

impl NotificationDispatchRegistrationSqlDbRepository {
    fn pooled_connection(
        &self,
    ) -> Result<
        PooledConnection<ConnectionManager<SqliteConnection>>,
        MappedErrors,
    > {
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

        conn.immediate_transaction(|conn| register_within_quota(conn, &batch))
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
            notification_dispatch::table.filter(
                notification_dispatch::created_at
                    .lt(dispatch_timestamp_to_text(&cutoff)),
            ),
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

// ? ---------------------------------------------------------------------------
// ? TESTS
// ? ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        repositories::message::{
            LocalMessageReadSqlDbRepository, LocalMessageWriteSqlDbRepository,
        },
        schema::message_queue,
        test_support::{setup_temp_db, TempDb},
        types::uuid_to_text,
    };

    use chrono::Duration;
    use diesel::connection::SimpleConnection;
    use myc_core::domain::{
        dtos::{
            email::Email,
            message::{FromEmail, Message, MessageStatus},
            notification_dispatch::NotificationDispatchIssuer,
        },
        entities::{LocalMessageReading, LocalMessageWrite},
    };
    use mycelium_base::entities::FetchManyResponseKind;
    use std::sync::Barrier;
    use uuid::Uuid;

    const TAG: &str = "service-01";

    const DOWN_MIGRATION: &str = include_str!(
        "../../../migrations/2026-11-20-000000_notification_dispatch/down.sql"
    );

    fn repository(
        db: &TempDb,
    ) -> NotificationDispatchRegistrationSqlDbRepository {
        NotificationDispatchRegistrationSqlDbRepository {
            db_config: db.provider.clone(),
        }
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

    fn dispatched(db: &TempDb, tag: &str) -> i64 {
        notification_dispatch::table
            .filter(notification_dispatch::sender_tag.eq(tag))
            .count()
            .get_result(&mut db.provider.get_pool().get().unwrap())
            .unwrap()
    }

    fn queued(db: &TempDb, ids: &[Uuid]) -> i64 {
        let ids: Vec<String> = ids.iter().map(uuid_to_text).collect();

        message_queue::table
            .filter(message_queue::id.eq_any(ids))
            .count()
            .get_result(&mut db.provider.get_pool().get().unwrap())
            .unwrap()
    }

    fn age_dispatches(db: &TempDb, tag: &str, age: Duration) {
        diesel::update(
            notification_dispatch::table
                .filter(notification_dispatch::sender_tag.eq(tag)),
        )
        .set(
            notification_dispatch::created_at
                .eq(dispatch_timestamp_to_text(&(Utc::now() - age))),
        )
        .execute(&mut db.provider.get_pool().get().unwrap())
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
    async fn batch_under_quota_is_traced_and_queued() -> Result<(), MappedErrors>
    {
        let db = setup_temp_db();
        let (rows, messages) = batch(2);
        let expected_ids: Vec<Uuid> = rows.iter().map(|row| row.id).collect();
        let issuer = rows[0].issuer.clone();

        let outcome = repository(&db)
            .register_and_enqueue(TAG, 5, rows, messages)
            .await?;

        assert_eq!(accepted_ids(outcome), expected_ids);
        assert_eq!(dispatched(&db, TAG), 2);
        assert_eq!(queued(&db, &expected_ids), 2);

        let stored = notification_dispatch::table
            .find(uuid_to_text(&expected_ids[0]))
            .select((
                notification_dispatch::channel,
                notification_dispatch::connection_string_id,
                notification_dispatch::account_id,
                notification_dispatch::tenant_id,
                notification_dispatch::recipient,
                notification_dispatch::template,
                notification_dispatch::locale,
            ))
            .get_result::<(
                String,
                i32,
                String,
                Option<String>,
                String,
                String,
                String,
            )>(&mut db.provider.get_pool().get().unwrap())
            .unwrap();

        assert_eq!(
            stored,
            (
                "email".to_string(),
                7,
                uuid_to_text(&issuer.account_id),
                issuer.tenant_id.map(|id| uuid_to_text(&id)),
                "user-00@example.com".to_string(),
                "invoice-ready".to_string(),
                "en-us".to_string(),
            )
        );

        Ok(())
    }

    #[tokio::test]
    async fn queued_message_is_readable_by_the_email_dispatcher(
    ) -> Result<(), MappedErrors> {
        let db = setup_temp_db();
        let ids = accepted_ids(send(&repository(&db), TAG, 5, 1).await?);

        let reader = LocalMessageReadSqlDbRepository {
            db_config: db.provider.clone(),
        };

        let FetchManyResponseKind::Found(claimed) = reader
            .list_oldest_messages(10, MessageStatus::Queued)
            .await?
        else {
            panic!("expected queued messages");
        };

        assert_eq!(claimed.len(), 1);
        assert_eq!(claimed[0].id, ids[0]);
        assert_eq!(claimed[0].message.subject, "Invoice ready");
        assert_eq!(claimed[0].message.to.email(), "user-00@example.com");

        Ok(())
    }

    #[tokio::test]
    async fn quota_boundary_accepts_at_quota_and_rejects_one_over(
    ) -> Result<(), MappedErrors> {
        let db = setup_temp_db();
        let repository = repository(&db);

        accepted_ids(send(&repository, TAG, 3, 2).await?);

        // 2 + 2 would cross the quota: the batch is refused as a whole.
        retry_after(send(&repository, TAG, 3, 2).await?);
        assert_eq!(dispatched(&db, TAG), 2);

        // 2 + 1 lands exactly on the quota.
        accepted_ids(send(&repository, TAG, 3, 1).await?);
        assert_eq!(dispatched(&db, TAG), 3);

        // One over.
        let (rows, messages) = batch(1);
        let refused_ids: Vec<Uuid> = rows.iter().map(|row| row.id).collect();
        let outcome = repository
            .register_and_enqueue(TAG, 3, rows, messages)
            .await?;

        retry_after(outcome);
        assert_eq!(dispatched(&db, TAG), 3);
        assert_eq!(queued(&db, &refused_ids), 0);

        // Another sender is not affected.
        accepted_ids(send(&repository, "service-02", 3, 3).await?);

        Ok(())
    }

    #[tokio::test]
    async fn retry_after_follows_the_oldest_dispatch_in_the_window(
    ) -> Result<(), MappedErrors> {
        let db = setup_temp_db();
        let repository = repository(&db);

        accepted_ids(send(&repository, TAG, 1, 1).await?);

        let fresh = retry_after(send(&repository, TAG, 1, 1).await?);
        assert!((3590..=3600).contains(&fresh), "{fresh}");

        age_dispatches(&db, TAG, Duration::minutes(59));

        let nearly_out = retry_after(send(&repository, TAG, 1, 1).await?);
        assert!((50..=60).contains(&nearly_out), "{nearly_out}");

        // Once the oldest dispatch leaves the window the quota is free again.
        age_dispatches(&db, TAG, Duration::minutes(61));

        accepted_ids(send(&repository, TAG, 1, 1).await?);

        Ok(())
    }

    #[tokio::test]
    async fn batch_larger_than_the_quota_never_reports_zero(
    ) -> Result<(), MappedErrors> {
        let db = setup_temp_db();

        let outcome = send(&repository(&db), TAG, 2, 3).await?;

        assert_eq!(retry_after(outcome), 3600);
        assert_eq!(dispatched(&db, TAG), 0);

        Ok(())
    }

    /// SQLite has a single writer and standalone runs a single pod, so the
    /// cross-pod proof of the Postgres adapter becomes: sends racing on
    /// separate connections of the one pool still never exceed the quota.
    #[test]
    fn concurrent_sends_never_exceed_the_quota() {
        const THREADS: usize = 4;
        const SENDS_PER_THREAD: usize = 5;
        const QUOTA: u32 = 7;

        let db = setup_temp_db();
        let repository = repository(&db);
        let barrier = Barrier::new(THREADS);

        let send_many = || {
            barrier.wait();

            (0..SENDS_PER_THREAD)
                .filter(|_| {
                    let outcome = tokio::runtime::Builder::new_current_thread()
                        .build()
                        .unwrap()
                        .block_on(send(&repository, TAG, QUOTA, 1))
                        .unwrap();

                    matches!(outcome, QuotaOutcome::Accepted(_))
                })
                .count()
        };

        let accepted: usize = std::thread::scope(|scope| {
            let workers: Vec<_> =
                (0..THREADS).map(|_| scope.spawn(send_many)).collect();

            workers
                .into_iter()
                .map(|worker| worker.join().unwrap())
                .sum()
        });

        assert_eq!(accepted, QUOTA as usize);
        assert_eq!(dispatched(&db, TAG), i64::from(QUOTA));
    }

    #[tokio::test]
    async fn failed_enqueue_rolls_the_dispatch_rows_back(
    ) -> Result<(), MappedErrors> {
        let db = setup_temp_db();
        let repository = repository(&db);
        let (rows, messages) = batch(2);
        let first_id = rows[0].id;

        // `message_queue.id` is a primary key here: queueing the second
        // message beforehand makes the batch's own insert of it fail.
        LocalMessageWriteSqlDbRepository {
            db_config: db.provider.clone(),
        }
        .send(messages[1].clone())
        .await?;

        let result = repository
            .register_and_enqueue(TAG, 5, rows, messages)
            .await;

        assert!(result.is_err());
        assert_eq!(dispatched(&db, TAG), 0);
        assert_eq!(queued(&db, &[first_id]), 0);

        // The failed attempt consumed no quota and left no lock behind.
        accepted_ids(send(&repository, TAG, 2, 2).await?);

        Ok(())
    }

    #[tokio::test]
    async fn unpaired_batch_writes_nothing() -> Result<(), MappedErrors> {
        let db = setup_temp_db();
        let (rows, _) = batch(1);
        let (_, other_messages) = batch(1);

        let result = repository(&db)
            .register_and_enqueue(TAG, 5, rows, other_messages)
            .await;

        assert!(result.is_err());
        assert_eq!(dispatched(&db, TAG), 0);

        Ok(())
    }

    #[tokio::test]
    async fn sender_tag_is_never_interpreted_as_sql() -> Result<(), MappedErrors>
    {
        let db = setup_temp_db();
        let repository = repository(&db);
        let tag = "x'; DROP TABLE notification_dispatch; --";

        accepted_ids(send(&repository, tag, 1, 1).await?);
        retry_after(send(&repository, tag, 1, 1).await?);

        assert_eq!(dispatched(&db, tag), 1);

        Ok(())
    }

    #[tokio::test]
    async fn purge_deletes_only_rows_older_than_the_cutoff(
    ) -> Result<(), MappedErrors> {
        let db = setup_temp_db();
        let repository = repository(&db);

        accepted_ids(send(&repository, "service-01", 5, 2).await?);
        accepted_ids(send(&repository, "service-02", 5, 1).await?);
        age_dispatches(&db, "service-01", Duration::days(91));
        age_dispatches(&db, "service-02", Duration::days(89));

        let purged = repository
            .purge_older_than(Utc::now() - Duration::days(90))
            .await?;

        assert_eq!(purged, 2);
        assert_eq!(dispatched(&db, "service-01"), 0);
        assert_eq!(dispatched(&db, "service-02"), 1);

        Ok(())
    }

    #[tokio::test]
    async fn down_migration_drops_the_table() -> Result<(), MappedErrors> {
        let db = setup_temp_db();
        let repository = repository(&db);

        accepted_ids(send(&repository, TAG, 5, 1).await?);

        db.provider
            .get_pool()
            .get()
            .unwrap()
            .batch_execute(DOWN_MIGRATION)
            .unwrap();

        assert!(send(&repository, TAG, 5, 1).await.is_err());

        Ok(())
    }
}
