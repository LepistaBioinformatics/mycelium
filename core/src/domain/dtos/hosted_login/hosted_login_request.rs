use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

/// What a client asked for when it sent the browser to the hosted login
/// (the OAuth 2.0 authorization request, RFC 6749 §4.1.1 + RFC 7636).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HostedLoginRequest {
    pub client_id: String,
    pub redirect_uri: String,

    /// Opaque value the client checks on return (CSRF on the client side).
    pub state: String,

    /// base64url(SHA-256(code_verifier)) — PKCE S256.
    pub code_challenge: String,
}

impl HostedLoginRequest {
    /// An S256 challenge is the base64url encoding of a SHA-256 digest:
    /// exactly 43 characters without padding.
    pub fn has_valid_code_challenge(&self) -> bool {
        self.code_challenge.len() == 43 && is_base64url(&self.code_challenge)
    }

    /// RFC 7636 §4.1: 43 to 128 characters from the unreserved set.
    pub fn is_valid_code_verifier(code_verifier: &str) -> bool {
        (43..=128).contains(&code_verifier.len())
            && code_verifier.chars().all(|c| {
                c.is_ascii_alphanumeric() || matches!(c, '-' | '.' | '_' | '~')
            })
    }

    pub fn pkce_matches(&self, code_verifier: &str) -> bool {
        let digest = Sha256::digest(code_verifier.as_bytes());
        let computed = URL_SAFE_NO_PAD.encode(digest);

        computed
            .as_bytes()
            .ct_eq(self.code_challenge.as_bytes())
            .into()
    }
}

fn is_base64url(value: &str) -> bool {
    value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    // RFC 7636 Appendix B test vector
    const VERIFIER: &str = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
    const CHALLENGE: &str = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM";

    fn request() -> HostedLoginRequest {
        HostedLoginRequest {
            client_id: "webapp".to_string(),
            redirect_uri: "https://app.example.com/callback".to_string(),
            state: "xyz".to_string(),
            code_challenge: CHALLENGE.to_string(),
        }
    }

    #[test]
    fn pkce_matches_rfc_7636_vector() {
        assert!(request().pkce_matches(VERIFIER));
    }

    #[test]
    fn pkce_rejects_other_verifier() {
        let other = format!("{}x", &VERIFIER[..VERIFIER.len() - 1]);

        assert!(!request().pkce_matches(&other));
        assert!(!request().pkce_matches(CHALLENGE));
    }

    #[test]
    fn validates_challenge_shape() {
        assert!(request().has_valid_code_challenge());

        let mut short = request();
        short.code_challenge = "abc".to_string();
        assert!(!short.has_valid_code_challenge());

        let mut padded = request();
        padded.code_challenge = format!("{}=", &CHALLENGE[..42]);
        assert!(!padded.has_valid_code_challenge());
    }

    #[test]
    fn validates_verifier_shape() {
        assert!(HostedLoginRequest::is_valid_code_verifier(VERIFIER));
        assert!(!HostedLoginRequest::is_valid_code_verifier("short"));
        assert!(!HostedLoginRequest::is_valid_code_verifier(
            &"a".repeat(129)
        ));
        assert!(!HostedLoginRequest::is_valid_code_verifier(&format!(
            "{} ",
            &VERIFIER[..42]
        )));
    }
}
