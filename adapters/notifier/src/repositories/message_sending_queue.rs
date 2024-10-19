use super::get_client;
use crate::settings::get_queue_config;

use async_trait::async_trait;
use myc_core::domain::{dtos::message::Message, entities::MessageSending};
use mycelium_base::{
    entities::CreateResponseKind,
    utils::errors::{creation_err, MappedErrors},
};
use redis::RedisError;
use serde::{Deserialize, Serialize};
use shaku::Component;
use tracing::{debug, error};
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct QueueMessage {
    pub(crate) message: Message,
    pub(crate) correspondence_key: Uuid,
}

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

        let config = get_queue_config().await;
        let correspondence_key = Uuid::new_v4();

        let message_string = match serde_json::to_string(&QueueMessage {
            correspondence_key: correspondence_key.to_owned(),
            message: message.to_owned(),
        }) {
            Ok(message) => message,
            Err(err) => {
                return creation_err(format!(
                    "Failed to build notification: {err}"
                ))
                .as_error()
            }
        };

        let res: Result<u32, RedisError> = redis::cmd("LPUSH")
            .arg(config.email_queue_name)
            .arg(message_string)
            .query(&mut connection);

        match res {
            Ok(res) => {
                debug!("New message sent to the queue: {res}");
                Ok(CreateResponseKind::Created(Some(correspondence_key)))
            }
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
