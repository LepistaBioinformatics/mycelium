use super::{email::Email, message::MessageSendingEvent};

use mycelium_base::utils::errors::{creation_err, MappedErrors};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Who asked for a notification: the connection string and its creator.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NotificationDispatchIssuer {
    /// The id of the notification connection string (`token.id`).
    pub connection_string_id: i32,

    /// The account that created the connection string (`AID`).
    pub account_id: Uuid,

    /// The tenant the connection string is bound to (`TID`).
    pub tenant_id: Option<Uuid>,
}

/// The trace of one notification sent to one recipient.
///
/// Template variables are deliberately absent: they may hold personal data
/// and are never stored.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NewNotificationDispatch {
    /// Shared with the queued message and the `X-Mycelium-Message-Id` header.
    pub id: Uuid,

    pub issuer: NotificationDispatchIssuer,

    pub recipient: Email,

    pub template: String,

    pub locale: String,
}

/// The answer of the hourly quota to a batch of notifications.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum QuotaOutcome {
    /// The whole batch was registered and queued, with these message ids.
    Accepted(Vec<Uuid>),

    /// Nothing was registered nor queued.
    #[serde(rename_all = "camelCase")]
    Exceeded { retry_after_secs: u64 },
}

/// The message ids of a batch, once every dispatch row is checked to trace
/// the message at the same position
///
/// Keeps the trace 1:1 with the queue: a dispatch without its message, or a
/// message without its dispatch, is refused before anything is written.
pub fn pair_dispatches_with_messages(
    rows: &[NewNotificationDispatch],
    messages: &[MessageSendingEvent],
) -> Result<Vec<Uuid>, MappedErrors> {
    let dispatch_ids: Vec<Uuid> = rows.iter().map(|row| row.id).collect();
    let message_ids: Vec<Uuid> =
        messages.iter().map(|message| message.id).collect();

    if dispatch_ids != message_ids {
        return creation_err(
            "Notification dispatch rows and messages are not paired by id",
        )
        .as_error();
    }

    Ok(message_ids)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::dtos::message::{FromEmail, Message};

    fn dispatch(id: Uuid) -> NewNotificationDispatch {
        NewNotificationDispatch {
            id,
            issuer: NotificationDispatchIssuer {
                connection_string_id: 7,
                account_id: Uuid::new_v4(),
                tenant_id: Some(Uuid::new_v4()),
            },
            recipient: Email::from_string("user@example.com".to_string())
                .unwrap(),
            template: "invoice-ready".to_string(),
            locale: "en-us".to_string(),
        }
    }

    fn message(id: Uuid) -> MessageSendingEvent {
        let mut event = MessageSendingEvent::new(Message {
            from: FromEmail::NamedEmail("Service 01".to_string()),
            to: Email::from_string("user@example.com".to_string()).unwrap(),
            cc: None,
            subject: "Subject".to_string(),
            body: "Body".to_string(),
            headers: vec![],
        });

        event.id = id;
        event
    }

    #[test]
    fn new_notification_dispatch_round_trips() {
        let row = dispatch(Uuid::new_v4());
        let json = serde_json::to_value(&row).unwrap();

        assert_eq!(json["issuer"]["connectionStringId"], 7);
        assert_eq!(json["recipient"]["domain"], "example.com");
        assert!(json.get("variables").is_none());

        let decoded: NewNotificationDispatch =
            serde_json::from_value(json).unwrap();

        assert_eq!(decoded, row);
    }

    #[test]
    fn quota_outcome_round_trips() {
        let exceeded = QuotaOutcome::Exceeded {
            retry_after_secs: 90,
        };
        let json = serde_json::to_value(&exceeded).unwrap();

        assert_eq!(json["exceeded"]["retryAfterSecs"], 90);

        for outcome in [exceeded, QuotaOutcome::Accepted(vec![Uuid::new_v4()])]
        {
            let json = serde_json::to_string(&outcome).unwrap();
            let decoded: QuotaOutcome = serde_json::from_str(&json).unwrap();

            assert_eq!(decoded, outcome);
        }
    }

    #[test]
    fn paired_batch_yields_the_message_ids() {
        let ids = vec![Uuid::new_v4(), Uuid::new_v4()];
        let rows: Vec<_> = ids.iter().map(|id| dispatch(*id)).collect();
        let messages: Vec<_> = ids.iter().map(|id| message(*id)).collect();

        assert_eq!(
            pair_dispatches_with_messages(&rows, &messages).unwrap(),
            ids
        );
    }

    #[test]
    fn unpaired_batch_is_refused() {
        let id = Uuid::new_v4();

        let missing_message =
            pair_dispatches_with_messages(&[dispatch(id)], &[]);
        let missing_dispatch =
            pair_dispatches_with_messages(&[], &[message(id)]);
        let other_id = pair_dispatches_with_messages(
            &[dispatch(id)],
            &[message(Uuid::new_v4())],
        );

        assert!(missing_message.is_err());
        assert!(missing_dispatch.is_err());
        assert!(other_id.is_err());
    }
}
