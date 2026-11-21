use crate::{
    models::{
        message::Message as MessageModel,
        notification_dispatch::NewNotificationDispatchRow,
    },
    schema::{
        message_queue as message_queue_model,
        notification_dispatch as notification_dispatch_model,
    },
};

use chrono::{DateTime, Utc};
use diesel::{
    dsl::{count_star, min},
    prelude::*,
    sql_types::Text,
    PgConnection,
};
use myc_core::domain::dtos::{
    notification_dispatch::{NewNotificationDispatch, QuotaOutcome},
    notification_quota_window::NotificationQuotaWindow,
};

/// Namespace of the advisory locks, so a sender tag never shares a lock key
/// with another feature locking on a plain name.
const LOCK_NAMESPACE: &str = "myc-notify:";

/// Held until the transaction ends. The key is a bound parameter.
const LOCK_SENDER_TAG: &str = "SELECT pg_advisory_xact_lock(hashtext($1))";

diesel::define_sql_function! {
    /// The wall clock, read after the lock is granted. `now()` is frozen at
    /// the transaction start, which may be long before a contended lock is
    /// granted.
    fn clock_timestamp() -> diesel::sql_types::Timestamptz;
}

/// The notifications of one send, ready to be written.
pub(super) struct NotificationBatch<'a> {
    pub sender_tag: &'a str,
    pub quota_per_hour: u32,
    pub dispatches: &'a [NewNotificationDispatch],
    pub queued_messages: &'a [MessageModel],
}

impl NotificationBatch<'_> {
    fn dispatch_rows(
        &self,
        created_at: DateTime<Utc>,
    ) -> Vec<NewNotificationDispatchRow> {
        self.dispatches
            .iter()
            .map(|dispatch| {
                NewNotificationDispatchRow::new(
                    self.sender_tag,
                    dispatch,
                    created_at,
                )
            })
            .collect()
    }
}

/// Count the sender's last hour and, when the batch fits, write the trace
/// rows and the queued messages
///
/// Must run inside a transaction: the per-sender lock serializes concurrent
/// sends of one tag across pods, and any failure rolls back both tables.
pub(super) fn register_within_quota(
    conn: &mut PgConnection,
    batch: &NotificationBatch,
) -> QueryResult<QuotaOutcome> {
    lock_sender_tag(conn, batch.sender_tag)?;

    let now =
        diesel::select(clock_timestamp()).get_result::<DateTime<Utc>>(conn)?;

    let exceeded = read_quota_window(conn, batch.sender_tag, now)?
        .retry_after_secs_if_exceeded(
            batch.quota_per_hour,
            batch.dispatches.len(),
            now,
        );

    if let Some(retry_after_secs) = exceeded {
        return Ok(QuotaOutcome::Exceeded { retry_after_secs });
    }

    accept_batch(conn, batch, now)
}

fn lock_sender_tag(
    conn: &mut PgConnection,
    sender_tag: &str,
) -> QueryResult<()> {
    diesel::sql_query(LOCK_SENDER_TAG)
        .bind::<Text, _>(format!("{LOCK_NAMESPACE}{sender_tag}"))
        .execute(conn)?;

    Ok(())
}

fn read_quota_window(
    conn: &mut PgConnection,
    sender_tag: &str,
    now: DateTime<Utc>,
) -> QueryResult<NotificationQuotaWindow> {
    let (dispatched, oldest) = notification_dispatch_model::table
        .filter(notification_dispatch_model::sender_tag.eq(sender_tag))
        .filter(
            notification_dispatch_model::created_at
                .gt(NotificationQuotaWindow::opens_at(now)),
        )
        .select((count_star(), min(notification_dispatch_model::created_at)))
        .get_result::<(i64, Option<DateTime<Utc>>)>(conn)?;

    Ok(NotificationQuotaWindow {
        dispatched: u64::try_from(dispatched).unwrap_or_default(),
        oldest,
    })
}

fn accept_batch(
    conn: &mut PgConnection,
    batch: &NotificationBatch,
    created_at: DateTime<Utc>,
) -> QueryResult<QuotaOutcome> {
    let rows = batch.dispatch_rows(created_at);

    diesel::insert_into(notification_dispatch_model::table)
        .values(&rows)
        .execute(conn)?;

    diesel::insert_into(message_queue_model::table)
        .values(batch.queued_messages)
        .execute(conn)?;

    Ok(QuotaOutcome::Accepted(
        rows.iter().map(|row| row.id).collect(),
    ))
}
