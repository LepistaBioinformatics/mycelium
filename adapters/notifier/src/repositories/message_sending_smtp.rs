use crate::settings::SMTP_CONFIG;

use async_trait::async_trait;
use lettre::{
    message::header::ContentType, transport::smtp::authentication::Credentials,
    Message as LettreMessage, SmtpTransport, Transport,
};
use myc_config::optional_config::OptionalConfig;
use myc_core::domain::{dtos::message::Message, entities::MessageSending};
use mycelium_base::{
    entities::CreateResponseKind,
    utils::errors::{creation_err, MappedErrors},
};
use shaku::Component;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = MessageSending)]
pub struct MessageSendingSmtpRepository {}

#[async_trait]
impl MessageSending for MessageSendingSmtpRepository {
    #[tracing::instrument(name = "MessageSendingSmtpRepository.send", skip_all)]
    async fn send(
        &self,
        message: Message,
    ) -> Result<CreateResponseKind<Option<Uuid>>, MappedErrors> {
        let binding = SMTP_CONFIG.lock().unwrap();
        let config = match binding.as_ref() {
            Some(config) => config,
            None => {
                return creation_err(
                    "Could not send email: SMTP config not found".to_string(),
                )
                .as_error()
            }
        };

        let config = match config {
            OptionalConfig::Disabled => {
                return Ok(CreateResponseKind::NotCreated(
                    None,
                    "SMTP config is disabled".to_string(),
                ))
            }
            OptionalConfig::Enabled(config) => config,
        };

        let email = LettreMessage::builder()
            .from(message.to_owned().from.get_email().parse().unwrap())
            .to(message.to_owned().to.get_email().parse().unwrap())
            .subject(message.to_owned().subject)
            .header(ContentType::TEXT_HTML)
            .body(message.to_owned().body)
            .unwrap();

        let credentials = Credentials::new(
            config.username.get_or_error()?.to_owned(),
            config.password.get_or_error()?.to_owned(),
        );

        let mailer = SmtpTransport::relay(&config.host.to_owned())
            .unwrap()
            .credentials(credentials)
            .build();

        match mailer.send(&email) {
            Ok(_) => Ok(CreateResponseKind::Created(None)),
            Err(err) => {
                creation_err(format!("Could not send email: {err}")).as_error()
            }
        }
    }
}
