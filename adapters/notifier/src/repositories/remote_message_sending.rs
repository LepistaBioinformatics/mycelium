use crate::models::ClientProvider;

use async_trait::async_trait;
use lettre::{
    message::header::ContentType, Message as LettreMessage, Transport,
};
use myc_core::domain::{
    dtos::message::{FromEmail, Message},
    entities::RemoteMessageWrite,
};
use mycelium_base::{
    entities::CreateResponseKind,
    utils::errors::{creation_err, MappedErrors},
};
use shaku::Component;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = RemoteMessageWrite)]
pub struct RemoteMessageSendingRepository {
    #[shaku(inject)]
    client: Arc<dyn ClientProvider>,
}

#[async_trait]
impl RemoteMessageWrite for RemoteMessageSendingRepository {
    #[tracing::instrument(name = "send", skip_all)]
    async fn send(
        &self,
        message: Message,
    ) -> Result<CreateResponseKind<Option<Uuid>>, MappedErrors> {
        let connection = self.client.get_smtp_client().as_ref().clone();

        let from_addr = (match message.to_owned().from {
            FromEmail::Email(email) => email.email(),
            FromEmail::NamedEmail(named_email) => named_email,
        })
        .parse()
        .map_err(|e| creation_err(format!("Invalid from email address: {e}")))?;

        let to_addr = message
            .to_owned()
            .to
            .email()
            .parse()
            .map_err(|e| creation_err(format!("Invalid to email address: {e}")))?;

        let email = LettreMessage::builder()
            .from(from_addr)
            .to(to_addr)
            .subject(message.to_owned().subject)
            .header(ContentType::TEXT_HTML)
            .body(message.to_owned().body)
            .map_err(|e| creation_err(format!("Could not build email message: {e}")))?;

        match connection.send(&email) {
            Ok(_) => Ok(CreateResponseKind::Created(None)),
            Err(err) => {
                creation_err(format!("Could not send email: {err}")).as_error()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use mycelium_base::utils::errors::creation_err;

    #[test]
    fn test_malformed_from_email_parse_returns_err() {
        let result: Result<lettre::Address, _> = "not@@an-email".parse();
        assert!(result.is_err());
        let mapped =
            result.map_err(|e| creation_err(format!("Invalid from email: {e}")));
        assert!(mapped.is_err());
    }

    #[test]
    fn test_malformed_to_email_parse_returns_err() {
        let result: Result<lettre::Address, _> = "also@@invalid".parse();
        assert!(result.is_err());
        let mapped =
            result.map_err(|e| creation_err(format!("Invalid to email: {e}")));
        assert!(mapped.is_err());
    }

    #[test]
    fn test_valid_email_parse_returns_ok() {
        let result: Result<lettre::Address, _> = "user@example.com".parse();
        assert!(result.is_ok());
    }
}
