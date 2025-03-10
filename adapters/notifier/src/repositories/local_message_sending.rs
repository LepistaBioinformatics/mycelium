use crate::models::ClientProvider;

use async_trait::async_trait;
use myc_core::domain::{
    dtos::message::{Message, MessageSendingEvent},
    entities::LocalMessageWrite,
};
use mycelium_base::{
    entities::CreateResponseKind,
    utils::errors::{creation_err, execution_err, MappedErrors},
};
use redis::RedisError;
use serde::{Deserialize, Serialize};
use shaku::Component;
use std::sync::Arc;
use tracing::{debug, error};
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct QueueMessage {
    pub(crate) message: Message,
    pub(crate) correspondence_key: Uuid,
}

#[derive(Component)]
#[shaku(interface = LocalMessageWrite)]
pub struct LocalMessageSendingRepository {
    #[shaku(inject)]
    notifier_provider: Arc<dyn ClientProvider>,
}

#[async_trait]
impl LocalMessageWrite for LocalMessageSendingRepository {
    #[tracing::instrument(name = "send", skip_all)]
    async fn send(
        &self,
        message_event: MessageSendingEvent,
    ) -> Result<CreateResponseKind<Option<Uuid>>, MappedErrors> {
        let mut connection =
            self.notifier_provider.get_redis_client().as_ref().clone();
        let correspondence_key = Uuid::new_v4();

        let message_string = match serde_json::to_string(&QueueMessage {
            correspondence_key: correspondence_key.to_owned(),
            message: message_event.message.to_owned(),
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
            .arg(
                self.notifier_provider
                    .get_queue_config()
                    .email_queue_name
                    .async_get_or_error()
                    .await?,
            )
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

    #[tracing::instrument(name = "ping", skip_all)]
    async fn ping(&self) -> Result<(), MappedErrors> {
        let mut connection =
            self.notifier_provider.get_redis_client().as_ref().clone();

        let res: Result<String, RedisError> =
            redis::cmd("PING").query(&mut connection);

        match res {
            Ok(res) => {
                debug!("Ping response: {res}");
                Ok(())
            }
            Err(err) => {
                error!("Failed to ping the redis server: {err}");

                return execution_err(format!(
                    "Failed to build notification: {err}"
                ))
                .as_error();
            }
        }
    }

    async fn delete_message_event(&self, _: Uuid) -> Result<(), MappedErrors> {
        unimplemented!(
            "Delete message event is not implemented for LocalMessageSendingRepository"
        );
    }

    async fn update_message_event(
        &self,
        _: MessageSendingEvent,
    ) -> Result<(), MappedErrors> {
        unimplemented!("Update message event is not implemented for LocalMessageSendingRepository");
    }
}
