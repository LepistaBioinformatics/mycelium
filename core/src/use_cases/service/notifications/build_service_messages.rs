use super::build_service_sender;
use crate::{
    domain::dtos::{
        email::Email,
        message::{Message, MessageSendingEvent},
        notification_dispatch::{
            NewNotificationDispatch, NotificationDispatchIssuer,
        },
        service_notification_config::ServiceNotificationConfig,
    },
    models::{AccountLifeCycle, RenderedServiceEmail, SenderTagMode},
};

use mycelium_base::utils::errors::MappedErrors;
use uuid::Uuid;

/// What every message of a service email batch shares.
pub(crate) struct ServiceEmailDraft<'a> {
    pub config: &'a ServiceNotificationConfig,
    pub issuer: NotificationDispatchIssuer,
    pub template: String,
    pub rendered: RenderedServiceEmail,
}

/// Build one message and one dispatch row per recipient
///
/// Each recipient gets its own message, so recipients never see each other,
/// and its own id, shared by the queued message, the dispatch row and the
/// `X-Mycelium-Message-Id` header.
#[tracing::instrument(name = "build_service_messages", skip_all)]
pub(crate) async fn build_service_messages(
    draft: &ServiceEmailDraft<'_>,
    recipients: Vec<Email>,
    mode: SenderTagMode,
    life_cycle: &AccountLifeCycle,
) -> Result<
    (Vec<NewNotificationDispatch>, Vec<MessageSendingEvent>),
    MappedErrors,
> {
    let mut rows = Vec::with_capacity(recipients.len());
    let mut messages = Vec::with_capacity(recipients.len());

    for recipient in recipients {
        let id = Uuid::new_v4();
        let message =
            build_message(draft, recipient.to_owned(), id, mode, life_cycle)
                .await?;

        messages.push(message);
        rows.push(build_row(draft, recipient, id));
    }

    Ok((rows, messages))
}

async fn build_message(
    draft: &ServiceEmailDraft<'_>,
    recipient: Email,
    id: Uuid,
    mode: SenderTagMode,
    life_cycle: &AccountLifeCycle,
) -> Result<MessageSendingEvent, MappedErrors> {
    let (from, headers) =
        build_service_sender(mode, life_cycle, draft.config, id).await?;

    let mut event = MessageSendingEvent::new(Message {
        from,
        to: recipient,
        cc: None,
        subject: draft.rendered.subject.to_owned(),
        body: draft.rendered.body.to_owned(),
        headers,
    });

    event.id = id;

    Ok(event)
}

fn build_row(
    draft: &ServiceEmailDraft<'_>,
    recipient: Email,
    id: Uuid,
) -> NewNotificationDispatch {
    NewNotificationDispatch {
        id,
        issuer: draft.issuer.to_owned(),
        recipient,
        template: draft.template.to_owned(),
        locale: draft.rendered.locale.to_owned(),
    }
}
