use myc_core::domain::{dtos::message::Message, entities::MessageSending};

use async_trait::async_trait;
use clean_base::{
    entities::CreateResponseKind,
    utils::errors::{factories::creation_err, MappedErrors},
};
use lettre::{
    message::header::ContentType, transport::smtp::authentication::Credentials,
    Message as LettreMessage, SmtpTransport, Transport,
};
use shaku::Component;

#[derive(Component)]
#[shaku(interface = MessageSending)]
pub struct MessageSendingSqlDbRepository {}

#[async_trait]
impl MessageSending for MessageSendingSqlDbRepository {
    async fn send(
        &self,
        message: Message,
    ) -> Result<CreateResponseKind<Message>, MappedErrors> {
        let email = LettreMessage::builder()
            .from(message.to_owned().from.get_email().parse().unwrap())
            .to(message.to_owned().to.get_email().parse().unwrap())
            .subject(message.to_owned().subject)
            .header(ContentType::TEXT_HTML)
            .body(message.to_owned().message_body)
            .unwrap();

        let credentials = Credentials::new(
            "elias.samuel.galvao@gmail.com".to_owned(),
            "zmbbuouogbpwwiav".to_owned(),
        );

        let mailer = SmtpTransport::relay("smtp.gmail.com")
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
