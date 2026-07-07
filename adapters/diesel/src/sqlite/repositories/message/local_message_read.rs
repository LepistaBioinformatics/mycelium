use crate::sqlite::{
    config::SqliteDbPoolProvider, models::message::Message as MessageModel,
    schema::message_queue, types::uuid_from_text,
};

use async_trait::async_trait;
use base64::{engine::general_purpose, Engine};
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
    pub db_config: Arc<dyn SqliteDbPoolProvider>,
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

        let messages = message_queue::table
            .order(message_queue::created.desc())
            .filter(message_queue::status.eq(status.to_string()))
            .limit(tail_size as i64)
            .load::<MessageModel>(conn)
            .map_err(|e| {
                creation_err(format!("Failed to list messages: {}", e))
            })?;

        let messages = messages
            .into_iter()
            .map(map_model_to_dto)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(FetchManyResponseKind::Found(messages))
    }
}

fn map_model_to_dto(
    message: MessageModel,
) -> Result<MessageSendingEvent, MappedErrors> {
    let message_string = general_purpose::STANDARD
        .decode(message.message)
        .map_err(|e| {
            creation_err(format!("Failed to decode message: {}", e))
                .with_code(NativeErrorCodes::MYC00002)
        })?;

    let message_string = String::from_utf8(message_string).map_err(|e| {
        creation_err(format!("Failed to decode message: {}", e))
            .with_code(NativeErrorCodes::MYC00002)
    })?;

    let serde_message = serde_json::from_str(&message_string).map_err(|e| {
        creation_err(format!("Failed to deserialize message: {}", e))
            .with_code(NativeErrorCodes::MYC00002)
    })?;

    Ok(MessageSendingEvent {
        id: uuid_from_text(&message.id).unwrap(),
        message: serde_message,
        created: crate::sqlite::types::timestamp_from_text(&message.created)
            .unwrap()
            .with_timezone(&chrono::Local),
        attempted: message.attempted.map(|dt| {
            crate::sqlite::types::timestamp_from_text(&dt)
                .unwrap()
                .with_timezone(&chrono::Local)
        }),
        status: MessageStatus::from_str(&message.status).unwrap_or_default(),
        attempts: message.attempts,
        error: message.error,
    })
}
