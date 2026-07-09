use crate::{
    config::SqliteDbPoolProvider,
    models::message::Message as MessageModel,
    schema::message_queue,
    types::{timestamp_to_text, uuid_to_text},
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
    pub db_config: Arc<dyn SqliteDbPoolProvider>,
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

        let message_queue_row = MessageModel {
            id: uuid_to_text(&message_event.id),
            message: message_base64,
            created: timestamp_to_text(
                &message_event.created.with_timezone(&chrono::Utc),
            ),
            attempted: message_event
                .attempted
                .map(|d| timestamp_to_text(&d.with_timezone(&chrono::Utc))),
            status: message_event.status.to_string(),
            attempts: message_event.attempts,
            error: message_event.error.clone(),
        };

        diesel::insert_into(message_queue::table)
            .values(message_queue_row)
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

        diesel::update(message_queue::table)
            .filter(message_queue::id.eq(uuid_to_text(&message_event.id)))
            .set((
                message_queue::attempted.eq(message_event.attempted.map(|d| {
                    timestamp_to_text(&d.with_timezone(&chrono::Utc))
                })),
                message_queue::status.eq(message_event.status.to_string()),
                message_queue::attempts.eq(message_event.attempts),
                message_queue::error.eq(message_event.error.clone()),
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

        diesel::delete(message_queue::table)
            .filter(message_queue::id.eq(uuid_to_text(&id)))
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

// ? ---------------------------------------------------------------------------
// ? TESTS
// ? ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        repositories::message::LocalMessageReadSqlDbRepository,
        test_support::setup_temp_db,
    };
    use chrono::Local;
    use myc_core::domain::{
        dtos::{
            email::Email,
            message::{FromEmail, Message, MessageStatus},
        },
        entities::LocalMessageReading,
    };
    use mycelium_base::entities::FetchManyResponseKind;

    #[tokio::test]
    async fn message_queue_round_trips_through_sqlite(
    ) -> Result<(), MappedErrors> {
        let db = setup_temp_db();
        let write = LocalMessageWriteSqlDbRepository {
            db_config: db.provider.clone(),
        };
        let read = LocalMessageReadSqlDbRepository {
            db_config: db.provider.clone(),
        };

        let message = Message {
            from: FromEmail::NamedEmail("Mycelium".into()),
            to: Email::from_string("owner@acme.test".into())?,
            cc: None,
            subject: "Welcome".into(),
            body: "<p>Hello</p>".into(),
        };
        let event = MessageSendingEvent::new(message);
        let event_id = event.id;

        write.send(event.clone()).await?;

        // Only queued messages are returned
        let queued =
            match read.list_oldest_messages(10, MessageStatus::Queued).await? {
                FetchManyResponseKind::Found(records) => records,
                _ => panic!("expected to find queued messages"),
            };
        assert_eq!(queued.len(), 1);
        assert_eq!(queued[0].id, event_id);
        assert_eq!(queued[0].message.subject, "Welcome");

        // Mark as sent
        let mut sent_event = event.clone();
        sent_event.status = MessageStatus::Sent;
        sent_event.attempts = 1;
        sent_event.attempted = Some(Local::now());
        write.update_message_event(sent_event).await?;

        let still_queued =
            read.list_oldest_messages(10, MessageStatus::Queued).await?;
        assert!(
            matches!(still_queued, FetchManyResponseKind::Found(ref v) if v.is_empty())
                || matches!(still_queued, FetchManyResponseKind::NotFound)
        );

        let sent =
            match read.list_oldest_messages(10, MessageStatus::Sent).await? {
                FetchManyResponseKind::Found(records) => records,
                _ => panic!("expected to find sent messages"),
            };
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].attempts, 1);

        // Delete
        write.delete_message_event(event_id).await?;
        let after_delete =
            read.list_oldest_messages(10, MessageStatus::Sent).await?;
        assert!(
            matches!(after_delete, FetchManyResponseKind::Found(ref v) if v.is_empty())
                || matches!(after_delete, FetchManyResponseKind::NotFound)
        );

        Ok(())
    }
}
