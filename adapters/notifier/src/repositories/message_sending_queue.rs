use super::get_client;

use async_trait::async_trait;
use myc_core::domain::{dtos::message::Message, entities::MessageSending};
use mycelium_base::{
    entities::CreateResponseKind,
    utils::errors::{creation_err, MappedErrors},
};
use redis::RedisError;
use shaku::Component;
use tracing::error;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = MessageSending)]
pub struct MessageSendingQueueRepository {}

#[async_trait]
impl MessageSending for MessageSendingQueueRepository {
    #[tracing::instrument(
        name = "MessageSendingQueueRepository.send",
        skip_all
    )]
    async fn send(
        &self,
        message: Message,
    ) -> Result<CreateResponseKind<Option<Uuid>>, MappedErrors> {
        let client = get_client().await;
        let mut connection = match client.get_connection() {
            Ok(conn) => conn,
            Err(err) => {
                return creation_err(format!(
                    "Failed to connect to the message queue: {err}"
                ))
                .as_error()
            }
        };

        let correspondence_key = Uuid::new_v4();

        let message_string = match serde_json::to_string(&message) {
            Ok(message) => message,
            Err(err) => {
                return creation_err(format!(
                    "Failed to build notification: {err}"
                ))
                .as_error()
            }
        };

        let res: Result<(), RedisError> = redis::cmd("SET")
            .arg(correspondence_key.to_owned().to_string())
            .arg(message_string)
            .query(&mut connection);

        match res {
            Ok(_) => Ok(CreateResponseKind::Created(Some(correspondence_key))),
            Err(err) => {
                error!(
                    "Failed to send notification to the message queue: {err}"
                );

                Ok(CreateResponseKind::NotCreated(
                    Some(correspondence_key),
                    "Notification not send".to_string(),
                ))
            }
        }
    }
}
