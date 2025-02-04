use std::sync::Arc;

use crate::models::ClientProvider;

use async_trait::async_trait;
use lettre::{
    message::header::ContentType, Message as LettreMessage, Transport,
};
use myc_core::domain::{
    dtos::message::{FromEmail, Message},
    entities::RemoteMessageSending,
};
use mycelium_base::{
    entities::CreateResponseKind,
    utils::errors::{creation_err, MappedErrors},
};
use shaku::Component;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = RemoteMessageSending)]
pub struct RemoteMessageSendingRepository {
    #[shaku(inject)]
    client: Arc<dyn ClientProvider>,
}

#[async_trait]
impl RemoteMessageSending for RemoteMessageSendingRepository {
    #[tracing::instrument(name = "send", skip_all)]
    async fn send(
        &self,
        message: Message,
    ) -> Result<CreateResponseKind<Option<Uuid>>, MappedErrors> {
        let connection = self.client.get_smtp_client().as_ref().clone();

        let email = LettreMessage::builder()
            .from(
                (match message.to_owned().from {
                    FromEmail::Email(email) => email.email(),
                    FromEmail::NamedEmail(named_email) => named_email,
                })
                .parse()
                .unwrap(),
            )
            .to(message.to_owned().to.email().parse().unwrap())
            .subject(message.to_owned().subject)
            .header(ContentType::TEXT_HTML)
            .body(message.to_owned().body)
            .unwrap();

        match connection.send(&email) {
            Ok(_) => Ok(CreateResponseKind::Created(None)),
            Err(err) => {
                creation_err(format!("Could not send email: {err}")).as_error()
            }
        }
    }
}
