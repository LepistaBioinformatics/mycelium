use super::dispatch_timestamp_to_text::dispatch_timestamp_to_text;
use crate::{
    models::{
        message::Message as MessageModel,
        notification_dispatch::NewNotificationDispatchRow,
    },
    schema::{message_queue, notification_dispatch},
    types::{timestamp_from_text, uuid_to_text},
};

use chrono::{DateTime, Utc};
use diesel::{
    dsl::{count_star, min},
    prelude::*,
    result::Error as DieselError,
    SqliteConnection,
};
use myc_core::domain::dtos::{
    notification_dispatch::{NewNotificationDispatch, QuotaOutcome},
    notification_quota_window::NotificationQuotaWindow,
};

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
            .map(|dispatch| self.dispatch_row(dispatch, created_at))
            .collect()
    }

    fn dispatch_row(
        &self,
        dispatch: &NewNotificationDispatch,
        created_at: DateTime<Utc>,
    ) -> NewNotificationDispatchRow {
        NewNotificationDispatchRow {
            id: uuid_to_text(&dispatch.id),
            sender_tag: self.sender_tag.to_owned(),
            connection_string_id: dispatch.issuer.connection_string_id,
            account_id: uuid_to_text(&dispatch.issuer.account_id),
            tenant_id: dispatch.issuer.tenant_id.map(|id| uuid_to_text(&id)),
            recipient: dispatch.recipient.email(),
            template: dispatch.template.to_owned(),
            locale: dispatch.locale.to_owned(),
            created_at: dispatch_timestamp_to_text(&created_at),
        }
    }
}

/// Count the sender's last hour and, when the batch fits, write the trace
/// rows and the queued messages
///
/// Must run inside a `BEGIN IMMEDIATE` transaction: it takes the single
/// SQLite write lock before the count, so no other send can slip between the
/// count and the inserts, and any failure rolls back both tables.
pub(super) fn register_within_quota(
    conn: &mut SqliteConnection,
    batch: &NotificationBatch,
) -> QueryResult<QuotaOutcome> {
    let now = Utc::now();

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

fn read_quota_window(
    conn: &mut SqliteConnection,
    sender_tag: &str,
    now: DateTime<Utc>,
) -> QueryResult<NotificationQuotaWindow> {
    let opens_at =
        dispatch_timestamp_to_text(&NotificationQuotaWindow::opens_at(now));

    let (dispatched, oldest) = notification_dispatch::table
        .filter(notification_dispatch::sender_tag.eq(sender_tag))
        .filter(notification_dispatch::created_at.gt(opens_at))
        .select((count_star(), min(notification_dispatch::created_at)))
        .get_result::<(i64, Option<String>)>(conn)?;

    Ok(NotificationQuotaWindow {
        dispatched: u64::try_from(dispatched).unwrap_or_default(),
        oldest: oldest.as_deref().map(decode_created_at).transpose()?,
    })
}

/// A `created_at` this adapter did not write is a corrupted row: fail the
/// send rather than guess a quota.
fn decode_created_at(text: &str) -> QueryResult<DateTime<Utc>> {
    timestamp_from_text(text)
        .map_err(|e| DieselError::DeserializationError(e.to_string().into()))
}

fn accept_batch(
    conn: &mut SqliteConnection,
    batch: &NotificationBatch,
    created_at: DateTime<Utc>,
) -> QueryResult<QuotaOutcome> {
    let rows = batch.dispatch_rows(created_at);

    diesel::insert_into(notification_dispatch::table)
        .values(&rows)
        .execute(conn)?;

    diesel::insert_into(message_queue::table)
        .values(batch.queued_messages)
        .execute(conn)?;

    let message_ids = batch.dispatches.iter().map(|dispatch| dispatch.id);

    Ok(QuotaOutcome::Accepted(message_ids.collect()))
}
