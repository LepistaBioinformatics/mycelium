use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::{rngs::OsRng, RngCore};

/// Generate an unguessable value for the hosted login flow (session id, CSRF
/// token, authorization code): 32 random bytes, base64url without padding.
pub fn generate_hosted_login_secret() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);

    URL_SAFE_NO_PAD.encode(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secrets_are_43_url_safe_chars_and_unique() {
        let first = generate_hosted_login_secret();
        let second = generate_hosted_login_secret();

        assert_eq!(first.len(), 43);
        assert!(first
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'));
        assert_ne!(first, second);
    }
}
