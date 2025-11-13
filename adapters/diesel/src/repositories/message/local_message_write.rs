use crate::{
    models::{config::DbPoolProvider, message::Message as MessageModel},
    schema::message_queue as message_queue_model,
};

use async_trait::async_trait;
use base64::{engine::general_purpose, Engine};
use diesel::prelude::*;
use myc_core::domain::{
    dtos::{
        message::MessageSendingEvent, native_error_codes::NativeErrorCodes,
    },
    entities::LocalMessageWrite,
};
use mycelium_base::{
    entities::CreateResponseKind,
    utils::errors::{creation_err, MappedErrors},
};
use shaku::Component;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = LocalMessageWrite)]
pub struct LocalMessageWriteSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn DbPoolProvider>,
}

#[async_trait]
impl LocalMessageWrite for LocalMessageWriteSqlDbRepository {
    #[tracing::instrument(name = "send", skip_all)]
    async fn send(
        &self,
        message_event: MessageSendingEvent,
    ) -> Result<CreateResponseKind<Option<Uuid>>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            creation_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let correspondence_key = message_event.id;

        let message_string = serde_json::to_string(&message_event.message)
            .map_err(|e| {
                creation_err(format!("Failed to serialize message: {}", e))
                    .with_code(NativeErrorCodes::MYC00002)
            })?;

        let message_base64 = general_purpose::STANDARD.encode(message_string);

        let message_queue = MessageModel {
            id: message_event.id,
            message: message_base64,
            created: message_event.created.naive_utc(),
            attempted: message_event.attempted.map(|d| d.naive_utc()),
            status: message_event.status.to_string(),
            attempts: message_event.attempts,
            error: message_event.error.clone(),
        };

        diesel::insert_into(message_queue_model::table)
            .values(message_queue)
            .execute(conn)
            .map_err(|e| {
                creation_err(format!("Failed to insert message: {}", e))
            })?;

        Ok(CreateResponseKind::Created(Some(correspondence_key)))
    }

    async fn update_message_event(
        &self,
        message_event: MessageSendingEvent,
    ) -> Result<(), MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            creation_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        diesel::update(message_queue_model::table)
            .filter(message_queue_model::id.eq(message_event.id))
            .set((
                message_queue_model::attempted
                    .eq(message_event.attempted.map(|d| d.naive_utc())),
                message_queue_model::status
                    .eq(message_event.status.to_string()),
                message_queue_model::attempts.eq(message_event.attempts),
                message_queue_model::error.eq(message_event.error.clone()),
            ))
            .execute(conn)
            .map_err(|e| {
                creation_err(format!("Failed to update message: {}", e))
                    .with_code(NativeErrorCodes::MYC00003)
            })?;

        Ok(())
    }

    async fn delete_message_event(&self, id: Uuid) -> Result<(), MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            creation_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        diesel::delete(message_queue_model::table)
            .filter(message_queue_model::id.eq(id))
            .execute(conn)
            .map_err(|e| {
                creation_err(format!("Failed to delete message: {}", e))
            })?;

        Ok(())
    }

    async fn ping(&self) -> Result<(), MappedErrors> {
        unimplemented!(
            "Ping is not implemented for LocalMessageSendingSqlDbRepository"
        );
    }
}
