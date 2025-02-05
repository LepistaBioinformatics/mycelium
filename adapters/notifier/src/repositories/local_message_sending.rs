use crate::models::ClientProvider;

use async_trait::async_trait;
use myc_core::domain::{dtos::message::Message, entities::LocalMessageSending};
use mycelium_base::{
    entities::CreateResponseKind,
    utils::errors::{creation_err, MappedErrors},
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
#[shaku(interface = LocalMessageSending)]
pub struct LocalMessageSendingRepository {
    #[shaku(inject)]
    notifier_provider: Arc<dyn ClientProvider>,
}

#[async_trait]
impl LocalMessageSending for LocalMessageSendingRepository {
    #[tracing::instrument(name = "send", skip_all)]
    async fn send(
        &self,
        message: Message,
    ) -> Result<CreateResponseKind<Option<Uuid>>, MappedErrors> {
        let mut connection =
            self.notifier_provider.get_redis_client().as_ref().clone();
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
}
