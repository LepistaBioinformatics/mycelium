use chrono::Local;
use myc_core::domain::dtos::message::{MessageSendingEvent, MessageStatus};
use myc_core::domain::entities::{
    LocalMessageReading, LocalMessageWrite, RemoteMessageWrite,
};
use mycelium_base::entities::FetchManyResponseKind;
use mycelium_base::{
    entities::CreateResponseKind,
    utils::errors::{creation_err, MappedErrors},
};
use std::sync::Arc;
use uuid::Uuid;

/// Consumes messages from the message queue
///
/// This function consumes messages from the message queue sending by smtp.
#[tracing::instrument(
    name = "consume_messages",
    skip(
        local_message_read_repo,
        local_message_write_repo,
        remote_message_write_repo
    )
)]
pub async fn consume_messages(
    queue_name: String,
    local_message_read_repo: Arc<dyn LocalMessageReading>,
    local_message_write_repo: Arc<dyn LocalMessageWrite>,
    remote_message_write_repo: Arc<dyn RemoteMessageWrite>,
) -> Result<(i32, i32), MappedErrors> {
    let max_retries = 3;
    let mut retries = 0;
    let mut processed_messages_success = 0;
    let mut processed_messages_failed = vec![];

    //
    // Consume the queue up to the end
    //
    loop {
        //
        // Update retries counter and check if the maximum
        // number of retries was reached
        //
        retries += 1;

        if retries >= max_retries {
            break;
        }

        let events = match local_message_read_repo
            .list_oldest_messages(25, MessageStatus::Queued)
            .await?
        {
            FetchManyResponseKind::NotFound => break,
            FetchManyResponseKind::Found(messages) => messages,
            FetchManyResponseKind::FoundPaginated {
                records, count, ..
            } => {
                if count == 0 {
                    break;
                }

                records
            }
        };

        for event in events {
            let mut _message = event.to_owned();

            if let Err(err) =
                process_record(event, remote_message_write_repo.clone()).await
            {
                _message.attempted = Some(Local::now());
                _message.error = Some(err.to_string());
                _message.attempts += 1;

                if _message.attempts >= 5 {
                    _message.status = MessageStatus::Failed;
                }

                processed_messages_failed.push(_message.id);

                if let Err(err) = local_message_write_repo
                    .update_message_event(_message)
                    .await
                {
                    panic!("Failed to update message: {err}");
                }
            } else {
                if !processed_messages_failed.contains(&_message.id) {
                    processed_messages_failed = processed_messages_failed
                        .into_iter()
                        .filter(|id| id != &_message.id)
                        .collect();
                }

                processed_messages_success += 1;

                if let Err(err) = local_message_write_repo
                    .delete_message_event(_message.id)
                    .await
                {
                    panic!("Failed to delete message: {err}");
                }
            }
        }

        //
        // If there are failed messages, do not break the loop, try again
        //
        if !processed_messages_failed.is_empty() {
            continue;
        }

        //
        // If there are no failed messages, break the loop
        //
        break;
    }

    Ok((
        processed_messages_success,
        processed_messages_failed.len() as i32,
    ))
}

async fn process_record(
    record: MessageSendingEvent,
    message_sending_repo: Arc<dyn RemoteMessageWrite>,
) -> Result<Uuid, MappedErrors> {
    if let CreateResponseKind::NotCreated(_, _) =
        message_sending_repo.send(record.message).await?
    {
        return creation_err("Failed to send message")
            .with_exp_true()
            .as_error();
    }

    Ok(record.id)
}
