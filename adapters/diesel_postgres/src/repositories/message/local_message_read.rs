use crate::{
    models::{config::DbPoolProvider, message::Message as MessageModel},
    schema::message_queue as message_queue_model,
};

use async_trait::async_trait;
use base64::{engine::general_purpose, Engine};
use chrono::{Duration, Local, TimeZone, Utc};
use diesel::prelude::*;
use myc_core::domain::{
    dtos::{
        message::{MessageSendingEvent, MessageStatus},
        native_error_codes::NativeErrorCodes,
    },
    entities::LocalMessageReading,
};
use mycelium_base::{
    entities::FetchManyResponseKind,
    utils::errors::{creation_err, MappedErrors},
};
use shaku::Component;
use std::str::FromStr;
use std::sync::Arc;

#[derive(Component)]
#[shaku(interface = LocalMessageReading)]
pub struct LocalMessageReadSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn DbPoolProvider>,
    // Claim visibility window in seconds, seeded from `[queue]
    // visibilityTimeoutSecs` by the port layer (defaults to 240 via the
    // config). `#[shaku(default)]` yields 0 only if a builder omits it (no
    // production path does) -- see the claim query for the invariant.
    #[shaku(default)]
    pub visibility_timeout_secs: i64,
}

#[async_trait]
impl LocalMessageReading for LocalMessageReadSqlDbRepository {
    #[tracing::instrument(name = "list_oldest_messages", skip_all)]
    async fn list_oldest_messages(
        &self,
        tail_size: i32,
        status: MessageStatus,
    ) -> Result<FetchManyResponseKind<MessageSendingEvent>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            creation_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        // SPEC_DEVIATION: read-named port now issues an UPDATE (claim) to guarantee
        // at-most-one-pod delivery (SKIP LOCKED). Accepted to avoid a core MessageStatus
        // change; see .claude/specs/features/postgres-only-mode/design.md §3.2.

        // SAFETY INVARIANT (`visibility_timeout_secs`, from `[queue]`): this
        // MUST exceed the worst-case processing time of a WHOLE claimed batch,
        // not one message. `attempted` is stamped on all `tail_size` rows up
        // front, then they are sent sequentially; if a live-but-slow pod's
        // batch outlives this window another pod re-claims its un-sent rows and
        // double-sends. lettre's default SMTP timeout is ~60s/send, so the
        // default batch = 3 (`[queue] claimBatchSize`) and default window = 240
        // give ~180s worst case < 240s. RAISING the batch REQUIRES raising the
        // window proportionally.
        //
        // TRADE-OFF (also affects full mode): the window doubles as the retry
        // back-off for a transiently-FAILED send. Fast re-select and
        // dedup-safety are the same knob (both key off `attempted`), so they
        // cannot be independently short without a separate lease mechanism.
        // Tune both via `[queue]`. See design.md §3.2.

        let now = Utc::now().naive_utc();
        let cutoff = now - Duration::seconds(self.visibility_timeout_secs);

        let claimed = conn
            .transaction::<Vec<MessageModel>, diesel::result::Error, _>(
                |conn| {
                    let rows = message_queue_model::table
                        .filter(
                            message_queue_model::status.eq(status.to_string()),
                        )
                        .filter(
                            message_queue_model::attempted
                                .is_null()
                                .or(message_queue_model::attempted.lt(cutoff)),
                        )
                        .order(message_queue_model::created.desc())
                        .limit(tail_size as i64)
                        .for_update()
                        .skip_locked()
                        .load::<MessageModel>(conn)?;

                    let ids = rows.iter().map(|row| row.id).collect::<Vec<_>>();

                    if ids.is_empty() {
                        return Ok(vec![]);
                    }

                    diesel::update(
                        message_queue_model::table
                            .filter(message_queue_model::id.eq_any(&ids)),
                    )
                    .set(message_queue_model::attempted.eq(now))
                    .execute(conn)?;

                    Ok(rows)
                },
            )
            .map_err(|e| {
                creation_err(format!("Failed to claim messages: {}", e))
            })?;

        let messages = claimed
            .into_iter()
            .map(|message| self.map_model_to_dto(message))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(FetchManyResponseKind::Found(messages))
    }
}

impl LocalMessageReadSqlDbRepository {
    fn map_model_to_dto(
        &self,
        message: MessageModel,
    ) -> Result<MessageSendingEvent, MappedErrors> {
        let message_string = general_purpose::STANDARD
            .decode(message.message)
            .map_err(|e| {
                creation_err(format!("Failed to decode message: {}", e))
                    .with_code(NativeErrorCodes::MYC00002)
            })?;

        let message_string =
            String::from_utf8(message_string).map_err(|e| {
                creation_err(format!("Failed to decode message: {}", e))
                    .with_code(NativeErrorCodes::MYC00002)
            })?;

        let serde_message =
            serde_json::from_str(&message_string).map_err(|e| {
                creation_err(format!("Failed to deserialize message: {}", e))
                    .with_code(NativeErrorCodes::MYC00002)
            })?;

        Ok(MessageSendingEvent {
            id: message.id,
            message: serde_message,
            created: Local.from_utc_datetime(&message.created),
            attempted: message.attempted.map(|dt| Local.from_utc_datetime(&dt)),
            status: MessageStatus::from_str(&message.status)
                .unwrap_or_default(),
            attempts: message.attempts,
            error: message.error,
        })
    }
}
