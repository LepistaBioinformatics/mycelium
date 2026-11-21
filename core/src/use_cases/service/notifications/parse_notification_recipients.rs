use crate::domain::dtos::{email::Email, native_error_codes::NativeErrorCodes};

use mycelium_base::utils::errors::{use_case_err, MappedErrors};

/// Characters that separate or decorate addresses in a mail header. A single
/// bare address never needs them.
const ADDRESS_LIST_CHARS: [char; 5] = [',', ';', '<', '>', '"'];

/// Parse the requested recipients, refusing anything but a bare address
///
/// `Email::from_string` matches only a prefix of its input, so on its own it
/// would turn `victim@allowed.example\r\nBcc: other@evil.example` into
/// `victim@allowed.example` and silently drop the rest. Here an entry is
/// accepted only when:
///
/// - it carries no control character (CR, LF, tab, ...) and no address-list
///   punctuation, checked before trimming so a trailing CRLF is not erased;
/// - once surrounding spaces are trimmed, it has no inner whitespace;
/// - the parsed address is exactly the trimmed entry (ignoring case).
///
/// The error is MYC00038 and never repeats the rejected value.
pub(crate) fn parse_notification_recipients(
    recipients: &[String],
) -> Result<Vec<Email>, MappedErrors> {
    recipients
        .iter()
        .map(|recipient| parse_notification_recipient(recipient))
        .collect()
}

fn parse_notification_recipient(
    recipient: &str,
) -> Result<Email, MappedErrors> {
    if recipient.chars().any(is_forbidden_char) {
        return invalid_recipient();
    }

    let trimmed = recipient.trim();

    if trimmed.chars().any(char::is_whitespace) {
        return invalid_recipient();
    }

    let Ok(email) = Email::from_string(trimmed.to_owned()) else {
        return invalid_recipient();
    };

    if email.email() != trimmed.to_lowercase() {
        return invalid_recipient();
    }

    Ok(email)
}

fn is_forbidden_char(character: char) -> bool {
    character.is_control() || ADDRESS_LIST_CHARS.contains(&character)
}

fn invalid_recipient() -> Result<Email, MappedErrors> {
    use_case_err("Each recipient must be a single valid email address")
        .with_code(NativeErrorCodes::MYC00038)
        .with_exp_true()
        .as_error()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(recipient: &str) -> Result<Vec<Email>, MappedErrors> {
        parse_notification_recipients(&[recipient.to_string()])
    }

    fn assert_rejected(recipient: &str) {
        let error = parse(recipient).unwrap_err();

        assert!(error.is_in(vec![NativeErrorCodes::MYC00038]), "{error}");
        assert!(!format!("{error:?}").contains("evil"), "{error:?}");
    }

    #[test]
    fn accepts_bare_addresses() {
        let parsed = parse_notification_recipients(&[
            "user@allowed.example".to_string(),
            "  First.Last+tag@Allowed.Example ".to_string(),
        ])
        .unwrap();

        assert_eq!(parsed[0].email(), "user@allowed.example");
        assert_eq!(parsed[1].email(), "first.last+tag@allowed.example");
    }

    #[test]
    fn rejects_a_header_injection() {
        assert_rejected("victim@allowed.example\r\nBcc: other@evil.example");
        assert_rejected("victim@allowed.example\nBcc: other@evil.example");
        assert_rejected("victim@allowed.example\r\n");
        assert_rejected("victim@allowed.example\t");
    }

    #[test]
    fn rejects_an_address_list() {
        assert_rejected("a@allowed.example, b@evil.example");
        assert_rejected("a@allowed.example,b@evil.example");
        assert_rejected("a@allowed.example;b@evil.example");
        assert_rejected("a@allowed.example b@evil.example");
        assert_rejected("Name <a@evil.example>");
    }

    #[test]
    fn rejects_silent_truncation() {
        assert_rejected("a@allowed.example@evil.example");
        assert_rejected("a@allowed.example/evil");
        assert_rejected("a@allowed.example!");
        assert_rejected("not-an-address");
        assert_rejected("");
    }
}
