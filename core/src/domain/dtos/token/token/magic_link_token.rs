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
// Each wrong code submitted for an email counts as a failed attempt on every
// active record of that email. A record is discarded once it reaches
// `MAX_FAILED_MAGIC_LINK_ATTEMPTS`, which bounds brute force on the code.
//
// ? ---------------------------------------------------------------------------

use crate::domain::dtos::email::Email;

use rand::random;
use serde::{Deserialize, Serialize};
use subtle::ConstantTimeEq;
use uuid::Uuid;

/// Wrong codes tolerated before a magic link record is discarded.
pub const MAX_FAILED_MAGIC_LINK_ATTEMPTS: u8 = 5;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MagicLinkTokenMeta {
    pub email: Email,

    /// UUID placed in the email link. Set to `None` after the display page is
    /// opened (phase 1 consumption). Once `None`, the display link is invalid.
    pub token: Option<String>,

    /// 6-digit code shown on the display page. Consumed on verify (phase 2).
    pub code: String,

    /// Wrong codes submitted for this email while the record was active.
    #[serde(default)]
    pub failed_attempts: u8,
}

impl MagicLinkTokenMeta {
    pub fn new(email: Email) -> Self {
        Self {
            email,
            token: Some(Uuid::new_v4().to_string()),
            code: format!("{:06}", random::<u32>() % 1_000_000),
            failed_attempts: 0,
        }
    }

    pub fn code_matches(&self, candidate: &str) -> bool {
        self.code.as_bytes().ct_eq(candidate.as_bytes()).into()
    }

    /// Count one failed attempt. Returns `true` when the record has run out
    /// of attempts and must be discarded.
    pub fn register_failed_attempt(&mut self) -> bool {
        self.failed_attempts = self.failed_attempts.saturating_add(1);
        self.failed_attempts >= MAX_FAILED_MAGIC_LINK_ATTEMPTS
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
        assert_eq!(restored.failed_attempts, 0);
    }

    #[test]
    fn test_magic_link_token_meta_deserializes_records_without_attempts() {
        let restored: MagicLinkTokenMeta =
            serde_json::from_value(serde_json::json!({
                "email": { "username": "test", "domain": "example.com" },
                "token": null,
                "code": "123456",
            }))
            .unwrap();

        assert_eq!(restored.failed_attempts, 0);
    }

    #[test]
    fn test_magic_link_token_meta_code_matches_only_exact_code() {
        let meta = MagicLinkTokenMeta::new(test_email());
        let wrong = format!(
            "{:06}",
            (meta.code.parse::<u32>().unwrap() + 1) % 1_000_000
        );

        assert!(meta.code_matches(&meta.code));
        assert!(!meta.code_matches(&wrong));
        assert!(!meta.code_matches(""));
        assert!(!meta.code_matches(&format!("{}0", meta.code)));
    }

    #[test]
    fn test_magic_link_token_meta_exhausts_after_max_attempts() {
        let mut meta = MagicLinkTokenMeta::new(test_email());

        for _ in 1..MAX_FAILED_MAGIC_LINK_ATTEMPTS {
            assert!(!meta.register_failed_attempt());
        }

        assert!(meta.register_failed_attempt());
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
