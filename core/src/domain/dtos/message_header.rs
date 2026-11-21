use mycelium_base::utils::errors::{creation_err, MappedErrors};

/// Prefix every extra mail header of a `Message` must carry.
pub const MESSAGE_HEADER_NAME_PREFIX: &str = "X-Mycelium-";

/// Longest header name a mail line can hold without folding.
const MESSAGE_HEADER_NAME_MAX_LENGTH: usize = 76;

/// Check that a `Message` extra header is safe to hand to the mail transport
///
/// Mail header names are case-insensitive, so the `X-Mycelium-` prefix is
/// compared ignoring case: `x-mycelium-foo` is accepted exactly like
/// `X-Mycelium-Foo`, and no casing reaches a header outside the namespace.
///
/// Names are restricted to ASCII letters, digits and `-`. Values must not
/// carry any control character. Together these rules keep a CR or LF from
/// starting a new header line (email header injection).
pub fn check_message_header(
    name: &str,
    value: &str,
) -> Result<(), MappedErrors> {
    if !name_is_in_the_mycelium_namespace(name) {
        return creation_err(format!(
            "Mail header name not allowed, expected \
             {MESSAGE_HEADER_NAME_PREFIX}*: {name:?}"
        ))
        .as_error();
    }

    if value.chars().any(char::is_control) {
        return creation_err(format!(
            "Mail header {name} carries a control character"
        ))
        .as_error();
    }

    Ok(())
}

fn name_is_in_the_mycelium_namespace(name: &str) -> bool {
    let prefix_length = MESSAGE_HEADER_NAME_PREFIX.len();

    let Some(prefix) = name.get(..prefix_length) else {
        return false;
    };

    prefix.eq_ignore_ascii_case(MESSAGE_HEADER_NAME_PREFIX)
        && name.len() > prefix_length
        && name.len() <= MESSAGE_HEADER_NAME_MAX_LENGTH
        && name.chars().all(is_header_name_char)
}

fn is_header_name_char(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == '-'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_a_mycelium_header() {
        assert!(check_message_header(
            "X-Mycelium-Source-Service",
            "service-01"
        )
        .is_ok());
    }

    #[test]
    fn prefix_comparison_ignores_case() {
        assert!(check_message_header("x-mycelium-message-id", "1").is_ok());
        assert!(check_message_header("X-MYCELIUM-MESSAGE-ID", "1").is_ok());
    }

    #[test]
    fn rejects_names_outside_the_namespace() {
        for name in ["Bcc", "Subject", "X-Other-Header", "X-Mycelium", ""] {
            assert!(check_message_header(name, "value").is_err(), "{name}");
        }
    }

    #[test]
    fn rejects_a_bare_prefix() {
        assert!(check_message_header("X-Mycelium-", "value").is_err());
    }

    #[test]
    fn rejects_line_breaks_and_control_characters_in_the_name() {
        for name in [
            "X-Mycelium-A\r\nBcc",
            "X-Mycelium-A\nBcc",
            "X-Mycelium-A\rBcc",
            "X-Mycelium-A\0",
            "X-Mycelium-A\t",
        ] {
            assert!(check_message_header(name, "value").is_err(), "{name:?}");
        }
    }

    #[test]
    fn rejects_separators_and_non_ascii_in_the_name() {
        for name in ["X-Mycelium-A:B", "X-Mycelium-A B", "X-Mycelium-Á"] {
            assert!(check_message_header(name, "value").is_err(), "{name:?}");
        }
    }

    #[test]
    fn rejects_an_overlong_name() {
        let name = format!("X-Mycelium-{}", "a".repeat(80));

        assert!(check_message_header(&name, "value").is_err());
    }

    #[test]
    fn rejects_line_breaks_and_control_characters_in_the_value() {
        for value in [
            "service-01\r\nBcc: attacker@example.com",
            "service-01\nBcc: attacker@example.com",
            "service-01\rBcc: attacker@example.com",
            "service-01\0",
            "service-01\t",
            "service-01\u{85}",
        ] {
            let result = check_message_header("X-Mycelium-A", value);

            assert!(result.is_err(), "{value:?}");
        }
    }
}
