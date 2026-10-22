use crate::domain::dtos::email::Email;

use rand::{rngs::OsRng, Rng};
use serde::{Deserialize, Serialize};
use subtle::ConstantTimeEq;

/// Wrong codes tolerated before a hosted login session is discarded.
pub const MAX_FAILED_HOSTED_LOGIN_ATTEMPTS: u8 = 5;

/// Codes issued for the same email that are still within the session TTL and
/// were not verified. Exhausted sessions are kept until they expire, so they
/// keep counting: this bounds the guesses
/// (`MAX_PENDING_HOSTED_LOGIN_CODES_PER_EMAIL` ×
/// `MAX_FAILED_HOSTED_LOGIN_ATTEMPTS`) and the emails sent per address per
/// TTL window.
pub const MAX_PENDING_HOSTED_LOGIN_CODES_PER_EMAIL: u8 = 5;

/// The 6-digit code emailed to prove ownership of the address. It only works
/// inside the browser session that requested it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HostedLoginEmailCode {
    pub email: Email,
    pub code: String,
    pub failed_attempts: u8,
}

impl HostedLoginEmailCode {
    pub fn new(email: Email) -> Self {
        Self {
            email,
            code: format!("{:06}", OsRng.gen_range(0..1_000_000u32)),
            failed_attempts: 0,
        }
    }

    pub fn code_matches(&self, candidate: &str) -> bool {
        self.code.as_bytes().ct_eq(candidate.as_bytes()).into()
    }

    pub fn is_exhausted(&self) -> bool {
        self.failed_attempts >= MAX_FAILED_HOSTED_LOGIN_ATTEMPTS
    }

    /// Count one failed attempt. Returns `true` when the code has run out of
    /// attempts and the session must be discarded.
    pub fn register_failed_attempt(&mut self) -> bool {
        self.failed_attempts = self.failed_attempts.saturating_add(1);
        self.is_exhausted()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn email_code() -> HostedLoginEmailCode {
        HostedLoginEmailCode::new(Email {
            username: "user".to_string(),
            domain: "example.com".to_string(),
        })
    }

    #[test]
    fn code_is_six_digits() {
        for _ in 0..50 {
            let code = email_code().code;

            assert_eq!(code.len(), 6);
            assert!(code.chars().all(|c| c.is_ascii_digit()));
        }
    }

    #[test]
    fn code_matches_only_exact_code() {
        let email_code = email_code();

        assert!(email_code.code_matches(&email_code.code.clone()));
        assert!(!email_code.code_matches(""));
        assert!(!email_code.code_matches(&format!("{}0", email_code.code)));
    }

    #[test]
    fn exhausts_after_max_attempts() {
        let mut email_code = email_code();

        for _ in 1..MAX_FAILED_HOSTED_LOGIN_ATTEMPTS {
            assert!(!email_code.register_failed_attempt());
        }

        assert!(email_code.register_failed_attempt());
    }
}
