use crate::{
    models::message::Message as MessageModel,
    types::{timestamp_to_text, uuid_to_text},
};

use base64::{engine::general_purpose, Engine};
use chrono::Utc;
use myc_core::domain::dtos::{
    message::MessageSendingEvent, native_error_codes::NativeErrorCodes,
};
use mycelium_base::utils::errors::{creation_err, MappedErrors};

/// Encode every message event of a batch as `message_queue` rows
pub(super) fn encode_messages_for_queue(
    events: &[MessageSendingEvent],
) -> Result<Vec<MessageModel>, MappedErrors> {
    events.iter().map(encode_message_for_queue).collect()
}

/// Encode a message event as a `message_queue` row
///
/// Same format `LocalMessageWrite::send` stores (base64 of the JSON
/// `Message`), so the email dispatcher reads both alike.
fn encode_message_for_queue(
    event: &MessageSendingEvent,
) -> Result<MessageModel, MappedErrors> {
    let message_json = serde_json::to_string(&event.message).map_err(|e| {
        creation_err(format!("Failed to serialize message: {e}"))
            .with_code(NativeErrorCodes::MYC00002)
    })?;

    Ok(MessageModel {
        id: uuid_to_text(&event.id),
        message: general_purpose::STANDARD.encode(message_json),
        created: timestamp_to_text(&event.created.with_timezone(&Utc)),
        attempted: event
            .attempted
            .map(|moment| timestamp_to_text(&moment.with_timezone(&Utc))),
        status: event.status.to_string(),
        attempts: event.attempts,
        error: event.error.to_owned(),
    })
}
