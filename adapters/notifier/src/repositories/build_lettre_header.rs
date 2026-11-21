use lettre::message::header::{HeaderName, HeaderValue};
use myc_core::domain::dtos::message_header::check_message_header;
use mycelium_base::utils::errors::{creation_err, MappedErrors};

/// Turn a `Message` extra header into a lettre raw header
///
/// The pair is checked against the core rules first (`X-Mycelium-*` names
/// only, no control characters), so nothing read back from the queue can
/// inject a mail header line.
pub(crate) fn build_lettre_header(
    name: &str,
    value: &str,
) -> Result<HeaderValue, MappedErrors> {
    check_message_header(name, value)?;

    let header_name =
        HeaderName::new_from_ascii(name.to_owned()).map_err(|e| {
            creation_err(format!("Invalid mail header name {name:?}: {e}"))
        })?;

    Ok(HeaderValue::new(header_name, value.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_a_mycelium_header() {
        let result =
            build_lettre_header("X-Mycelium-Source-Service", "service-01");

        assert!(result.is_ok());
    }

    #[test]
    fn refuses_a_name_outside_the_namespace() {
        assert!(build_lettre_header("Bcc", "attacker@example.com").is_err());
    }

    #[test]
    fn refuses_a_value_with_a_line_break() {
        let result = build_lettre_header(
            "X-Mycelium-Source-Service",
            "service-01\r\nBcc: attacker@example.com",
        );

        assert!(result.is_err());
    }
}
