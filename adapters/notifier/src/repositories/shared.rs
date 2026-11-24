use super::build_lettre_header::build_lettre_header;

use lettre::{
    message::{header::ContentType, MessageBuilder},
    Message as LettreMessage,
};
use myc_core::domain::dtos::message::{FromEmail, Message};
use mycelium_base::utils::errors::{creation_err, MappedErrors};

/// Format an SMTP send failure for the caller.
///
/// When the underlying error is the OpenSSL "wrong version number" record
/// mismatch, the client attempted an implicit-TLS handshake against a
/// plaintext/STARTTLS-only port -- append a hint pointing at the fix (#178).
pub(crate) fn smtp_send_error_message(err: impl ToString) -> String {
    let err = err.to_string();

    // Best-effort UX hint only. `lettre`/native-tls surface this TLS-record
    // mismatch as an OpenSSL text string (no granular error kind to match on),
    // so we substring-match. If the wording changes across OpenSSL versions the
    // hint silently disappears -- the underlying error is still reported, so
    // this degrades gracefully rather than breaking.
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

    let builder = LettreMessage::builder()
        .from(from_addr)
        .to(to_addr)
        .subject(message.to_owned().subject)
        .header(ContentType::TEXT_HTML);

    append_custom_headers(builder, &message.headers)?
        .body(message.to_owned().body)
        .map_err(|e| {
            creation_err(format!("Could not build email message: {e}"))
        })
}

/// Append the `Message` extra headers, refusing any that is not allowed.
fn append_custom_headers(
    builder: MessageBuilder,
    headers: &[(String, String)],
) -> Result<MessageBuilder, MappedErrors> {
    headers.iter().try_fold(builder, |builder, (name, value)| {
        Ok(builder.raw_header(build_lettre_header(name, value)?))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use myc_core::domain::dtos::email::Email;

    fn message_with_headers(headers: Vec<(&str, &str)>) -> Message {
        Message {
            from: FromEmail::NamedEmail(
                "\"Service 01 via Mycelium\" <noreply+service-01@example.com>"
                    .to_string(),
            ),
            to: Email::from_string("user@example.com".to_string()).unwrap(),
            cc: None,
            subject: "Invoice ready".to_string(),
            body: "<p>Hello</p>".to_string(),
            headers: headers
                .into_iter()
                .map(|(name, value)| (name.to_string(), value.to_string()))
                .collect(),
        }
    }

    fn formatted(message: &Message) -> String {
        let email = build_lettre_message(message).unwrap();
        String::from_utf8(email.formatted()).unwrap()
    }

    #[test]
    fn built_message_contains_the_custom_headers() {
        let message = message_with_headers(vec![
            ("X-Mycelium-Source-Service", "service-01"),
            ("X-Mycelium-Message-Id", "0a1b2c3d"),
        ]);

        let formatted = formatted(&message);

        assert!(formatted.contains("X-Mycelium-Source-Service: service-01\r\n"));
        assert!(formatted.contains("X-Mycelium-Message-Id: 0a1b2c3d\r\n"));
    }

    #[test]
    fn quoted_display_name_and_plus_address_reach_the_from_header() {
        let formatted = formatted(&message_with_headers(vec![]));

        assert!(
            formatted.contains("<noreply+service-01@example.com>\r\n"),
            "{formatted}"
        );
        assert!(formatted.contains("Service 01 via Mycelium"), "{formatted}");
    }

    #[test]
    fn escaped_quote_in_the_display_name_does_not_add_a_sender() {
        let mut message = message_with_headers(vec![]);
        message.from = FromEmail::NamedEmail(
            "\"A\\\" <attacker@example.com> via B\" <noreply@example.com>"
                .to_string(),
        );

        let email = build_lettre_message(&message).unwrap();

        assert_eq!(
            email.envelope().from().map(ToString::to_string),
            Some("noreply@example.com".to_string())
        );
    }

    #[test]
    fn message_without_headers_still_builds() {
        let formatted = formatted(&message_with_headers(vec![]));

        assert!(!formatted.contains("X-Mycelium-"));
    }

    #[test]
    fn disallowed_header_name_is_an_error() {
        let message =
            message_with_headers(vec![("Bcc", "attacker@example.com")]);

        assert!(build_lettre_message(&message).is_err());
    }

    #[test]
    fn header_injection_through_name_or_value_is_an_error() {
        let in_value = message_with_headers(vec![(
            "X-Mycelium-Source-Service",
            "service-01\r\nBcc: attacker@example.com",
        )]);
        let in_name = message_with_headers(vec![(
            "X-Mycelium-A\r\nBcc: attacker@example.com\r\nX-Mycelium-B",
            "value",
        )]);

        assert!(build_lettre_message(&in_value).is_err());
        assert!(build_lettre_message(&in_name).is_err());
    }

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
