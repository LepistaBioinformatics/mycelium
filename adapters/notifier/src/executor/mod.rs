use crate::repositories::{get_client, QueueMessage};
use myc_core::domain::entities::MessageSending;
use mycelium_base::{
    entities::CreateResponseKind,
    utils::errors::{creation_err, MappedErrors},
};
use redis::RedisError;
use uuid::Uuid;

/// Consumes messages from the message queue
///
/// This function consumes messages from the message queue sending by smtp.
#[tracing::instrument(name = "consume_messages", skip(message_sending_repo))]
pub async fn consume_messages(
    queue_name: String,
    message_sending_repo: Box<&dyn MessageSending>,
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

    let res: Result<String, RedisError> =
        redis::cmd("RPOP").arg(queue_name).query(&mut connection);

    let message_str = match res {
        Ok(res) => Some(res),
        Err(err) => {
            if err.is_cluster_error() {
                return creation_err(format!(
                    "Failed to consume notification to the message queue: {err}"
                ))
                .as_error();
            }

            if err.is_io_error() {
                return creation_err(format!(
                    "Failed to consume notification to the message queue: {err}"
                ))
                .as_error();
            }

            None
        }
    };

    if let Some(msg) = message_str {
        let message = match serde_json::from_str::<QueueMessage>(&msg) {
            Ok(msg) => msg,
            Err(err) => {
                return creation_err(format!(
                    "Failed to deserialize notification: {err}"
                ))
                .as_error()
            }
        };

        return message_sending_repo.send(message.message).await;
    }

    return creation_err("No message to consume")
        .with_exp_true()
        .as_error();
}
