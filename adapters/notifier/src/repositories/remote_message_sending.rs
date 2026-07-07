use crate::models::ClientProvider;
use crate::repositories::shared::build_lettre_message;

use async_trait::async_trait;
use lettre::Transport;
use myc_core::domain::{dtos::message::Message, entities::RemoteMessageWrite};
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
        let email = build_lettre_message(&message)?;

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
        let mapped = result
            .map_err(|e| creation_err(format!("Invalid from email: {e}")));
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
