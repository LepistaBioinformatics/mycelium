// ? ---------------------------------------------------------------------------
// ? MagicLinkTokenMeta
//
// Data type used during the magic link (passwordless) login procedure.
//
// Two-phase consumption:
//   Phase 1 (display): `token` UUID is consumed (set to None).
//                      The email link becomes single-use.
//   Phase 2 (verify):  `code` + `email` pair is consumed (record deleted).
//                      The 6-digit code is single-use.
//
// ? ---------------------------------------------------------------------------

use crate::domain::dtos::email::Email;

use rand::random;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MagicLinkTokenMeta {
    pub email: Email,

    /// UUID placed in the email link. Set to `None` after the display page is
    /// opened (phase 1 consumption). Once `None`, the display link is invalid.
    pub token: Option<String>,

    /// 6-digit code shown on the display page. Consumed on verify (phase 2).
    pub code: String,
}

impl MagicLinkTokenMeta {
    pub fn new(email: Email) -> Self {
        Self {
            email,
            token: Some(Uuid::new_v4().to_string()),
            code: format!("{:06}", random::<u32>() % 1_000_000),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::dtos::email::Email;

    fn test_email() -> Email {
        Email {
            username: "test".to_string(),
            domain: "example.com".to_string(),
        }
    }

    #[test]
    fn test_magic_link_token_meta_fields() {
        let meta = MagicLinkTokenMeta::new(test_email());

        assert_eq!(meta.email.username, "test");
        assert_eq!(meta.email.domain, "example.com");
        assert!(meta.token.is_some());
        assert_eq!(meta.code.len(), 6);
        assert!(
            meta.code.chars().all(|c| c.is_ascii_digit()),
            "code must be all digits"
        );
    }

    #[test]
    fn test_magic_link_token_meta_code_always_six_digits() {
        let email = test_email();
        for _ in 0..50 {
            let meta = MagicLinkTokenMeta::new(email.clone());
            assert_eq!(
                meta.code.len(),
                6,
                "code must always be exactly 6 digits"
            );
            assert!(
                meta.code.parse::<u32>().is_ok(),
                "code must be parseable as u32"
            );
        }
    }

    #[test]
    fn test_magic_link_token_meta_serialization_round_trip() {
        let meta = MagicLinkTokenMeta::new(test_email());
        let json = serde_json::to_string(&meta).unwrap();
        let restored: MagicLinkTokenMeta = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.email.username, meta.email.username);
        assert_eq!(restored.email.domain, meta.email.domain);
        assert_eq!(restored.token, meta.token);
        assert_eq!(restored.code, meta.code);
    }

    #[test]
    fn test_magic_link_token_meta_token_is_valid_uuid() {
        let meta = MagicLinkTokenMeta::new(test_email());
        let token_str = meta.token.unwrap();
        assert!(
            Uuid::parse_str(&token_str).is_ok(),
            "token must be a valid UUID"
        );
    }
}
