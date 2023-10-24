use crate::settings::SMTP_CONFIG;

use async_trait::async_trait;
use clean_base::{
    entities::CreateResponseKind,
    utils::errors::{factories::creation_err, MappedErrors},
};
use lettre::{
    message::header::ContentType, transport::smtp::authentication::Credentials,
    Message as LettreMessage, SmtpTransport, Transport,
};
use myc_core::domain::{dtos::message::Message, entities::MessageSending};
use shaku::Component;

#[derive(Component)]
#[shaku(interface = MessageSending)]
pub struct MessageSendingSmtpRepository {}

#[async_trait]
impl MessageSending for MessageSendingSmtpRepository {
    async fn send(
        &self,
        message: Message,
    ) -> Result<CreateResponseKind<Message>, MappedErrors> {
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

        let email = LettreMessage::builder()
            .from(message.to_owned().from.get_email().parse().unwrap())
            .to(message.to_owned().to.get_email().parse().unwrap())
            .subject(message.to_owned().subject)
            .header(ContentType::TEXT_HTML)
            .body(message.to_owned().message_body)
            .unwrap();

        let credentials = Credentials::new(
            config.username.to_owned(),
            config.password.to_owned(),
        );

        let mailer = SmtpTransport::relay(&config.host.to_owned())
            .unwrap()
            .credentials(credentials)
            .build();

        match mailer.send(&email) {
            Ok(_) => Ok(CreateResponseKind::Created(message)),
            Err(err) => {
                creation_err(format!("Could not send email: {err}")).as_error()
            }
        }
    }
}

// * ---------------------------------------------------------------------------
// * TESTS
// * ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::init_config_from_file;

    use myc_core::domain::dtos::email::Email;
    use std::{path::PathBuf, str::FromStr};

    #[tokio::test]
    async fn should_send_email() {
        let env_config_path = match std::env::var("SETTINGS_ENV_PATH") {
            Ok(path) => path,
            Err(err) => panic!("Error on get env : {err}"),
        };

        let env_test_email = match std::env::var("ENV_TEST_EMAIL") {
            Ok(path) => path,
            Err(err) => panic!("Error on get env var: {err}"),
        };

        let settings_env_path =
            match PathBuf::from_str(env_config_path.as_str()) {
                Ok(path) => path,
                Err(err) => panic!("Error on parse ENV_TEST_EMAIL: {err}"),
            };

        init_config_from_file(settings_env_path).await;

        let email = match Email::from_string(env_test_email) {
            Ok(email) => email,
            Err(err) => panic!("Error on parse email: {err}"),
        };

        let message = Message {
            from: email.to_owned(),
            to: email,
            cc: None,
            subject: "Teste".to_owned(),
            message_head: None,
            message_body: "Teste".to_owned(),
            message_footer: None,
        };

        let repo = MessageSendingSmtpRepository {};

        match repo.send(message).await.unwrap() {
            CreateResponseKind::Created(_) => (),
            _ => panic!("Error on send email"),
        };
    }
}
