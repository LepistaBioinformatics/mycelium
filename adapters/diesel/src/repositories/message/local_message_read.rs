use crate::{
    models::{config::DbPoolProvider, message::Message as MessageModel},
    schema::message_queue as message_queue_model,
};

use async_trait::async_trait;
use base64::{engine::general_purpose, Engine};
use chrono::{Local, TimeZone};
use diesel::prelude::*;
use myc_core::domain::{
    dtos::{
        message::{Message, MessageSendingEvent, MessageStatus},
        native_error_codes::NativeErrorCodes,
    },
    entities::LocalMessageReading,
};
use mycelium_base::{
    entities::FetchManyResponseKind,
    utils::errors::{creation_err, MappedErrors},
};
use serde::{Deserialize, Serialize};
use shaku::Component;
use std::str::FromStr;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct QueueMessage {
    pub(crate) message: Message,
    pub(crate) correspondence_key: Uuid,
}

#[derive(Component)]
#[shaku(interface = LocalMessageReading)]
pub struct LocalMessageReadSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn DbPoolProvider>,
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

        let messages = message_queue_model::table
            .order(message_queue_model::created.desc())
            .filter(message_queue_model::status.eq(status.to_string()))
            .limit(tail_size as i64)
            .load::<MessageModel>(conn)
            .map_err(|e| {
                creation_err(format!("Failed to list messages: {}", e))
            })?;

        let messages = messages
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
