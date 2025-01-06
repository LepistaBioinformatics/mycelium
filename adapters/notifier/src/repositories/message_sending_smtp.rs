use crate::settings::get_smtp_client_config;

use async_trait::async_trait;
use lettre::{
    message::header::ContentType, transport::smtp::authentication::Credentials,
    Message as LettreMessage, SmtpTransport, Transport,
};
use myc_config::optional_config::OptionalConfig;
use myc_core::domain::{
    dtos::message::{FromEmail, Message},
    entities::MessageSending,
};
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
        let config = get_smtp_client_config().await;

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

        //
        // Extract username and password into a boxed future
        //
        let username = config.username.async_get_or_error().await;
        let password = config.password.async_get_or_error().await;
        let credentials = Credentials::new(username?, password?);

        let mailer =
            SmtpTransport::relay(&config.host.async_get_or_error().await?)
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
