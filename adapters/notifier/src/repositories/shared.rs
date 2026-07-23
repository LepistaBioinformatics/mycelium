use lettre::{message::header::ContentType, Message as LettreMessage};
use myc_core::domain::dtos::message::{FromEmail, Message};
use mycelium_base::utils::errors::{creation_err, MappedErrors};

/// Format an SMTP send failure for the caller.
///
/// When the underlying error is the OpenSSL "wrong version number" record
/// mismatch, the client attempted an implicit-TLS handshake against a
/// plaintext/STARTTLS-only port -- append a hint pointing at the fix (#178).
pub(crate) fn smtp_send_error_message(err: impl ToString) -> String {
    let err = err.to_string();

    if err.contains("wrong version number") {
        return format!(
            "Could not send email: {err} (hint: the server likely expects \
             STARTTLS -- set `[smtp] security = \"starttls\"`, typically with \
             port 587)"
        );
    }

    format!("Could not send email: {err}")
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrong_version_number_error_gets_starttls_hint() {
        let message = smtp_send_error_message(
            "Connection error: error:0A00010B:SSL routines:ssl3_get_record:\
             wrong version number",
        );

        assert!(message.contains("Could not send email"));
        assert!(message.contains("security = \"starttls\""));
    }

    #[test]
    fn unrelated_error_has_no_hint() {
        let message = smtp_send_error_message("authentication failed");

        assert_eq!(message, "Could not send email: authentication failed");
        assert!(!message.contains("starttls"));
    }
}
