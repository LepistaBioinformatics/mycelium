use lettre::{message::header::ContentType, Message as LettreMessage};
use myc_core::domain::dtos::message::{FromEmail, Message};
use mycelium_base::utils::errors::{creation_err, MappedErrors};

pub(crate) fn build_lettre_message(
    message: &Message,
) -> Result<LettreMessage, MappedErrors> {
    let from_addr = (match message.to_owned().from {
        FromEmail::Email(email) => email.email(),
        FromEmail::NamedEmail(named_email) => named_email,
    })
    .parse()
    .map_err(|e| creation_err(format!("Invalid from email address: {e}")))?;

    let to_addr =
        message.to_owned().to.email().parse().map_err(|e| {
            creation_err(format!("Invalid to email address: {e}"))
        })?;

    LettreMessage::builder()
        .from(from_addr)
        .to(to_addr)
        .subject(message.to_owned().subject)
        .header(ContentType::TEXT_HTML)
        .body(message.to_owned().body)
        .map_err(|e| {
            creation_err(format!("Could not build email message: {e}"))
        })
}
