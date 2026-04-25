use super::types::WebhookSecret;

use secrecy::ExposeSecret;
use subtle::ConstantTimeEq;

/// Verify the X-Telegram-Bot-Api-Secret-Token header against the stored secret.
///
/// Returns `true` only when the header is present and matches the expected
/// value using constant-time comparison. Never panics on any input.
pub fn verify_webhook_secret(
    header: Option<&str>,
    expected: &WebhookSecret,
) -> bool {
    let Some(value) = header else {
        return false;
    };

    let expected_bytes = expected.0.expose_secret().as_bytes();
    let actual_bytes = value.as_bytes();

    if expected_bytes.len() != actual_bytes.len() {
        return false;
    }

    expected_bytes.ct_eq(actual_bytes).unwrap_u8() == 1
}

// ? ---------------------------------------------------------------------------
// ? Tests
// ? ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::SecretString;

    fn make_secret(value: &str) -> WebhookSecret {
        WebhookSecret(SecretString::new(value.into()))
    }

    #[test]
    fn correct_header_returns_true() {
        let secret = make_secret("correct-secret");
        assert!(verify_webhook_secret(Some("correct-secret"), &secret));
    }

    #[test]
    fn wrong_header_returns_false() {
        let secret = make_secret("correct-secret");
        assert!(!verify_webhook_secret(Some("wrong-secret"), &secret));
    }

    #[test]
    fn missing_header_returns_false() {
        let secret = make_secret("correct-secret");
        assert!(!verify_webhook_secret(None, &secret));
    }

    #[test]
    fn empty_header_returns_false() {
        let secret = make_secret("correct-secret");
        assert!(!verify_webhook_secret(Some(""), &secret));
    }
}
