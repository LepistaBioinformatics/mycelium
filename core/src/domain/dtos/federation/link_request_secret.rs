use crate::domain::dtos::hosted_login::generate_hosted_login_secret;

use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

/// The secret carried by a link initiation URL (FED-42)
///
/// Only its SHA-256 (hex) is stored, so a database read cannot replay an
/// initiation URL.
///
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinkRequestSecret(String);

impl LinkRequestSecret {
    pub fn mint() -> Self {
        Self(generate_hosted_login_secret())
    }

    pub fn from_url(value: String) -> Self {
        Self(value)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn hash(&self) -> String {
        hex::encode(Sha256::digest(self.0.as_bytes()))
    }

    pub fn matches(&self, stored_hash: &str) -> bool {
        self.hash().as_bytes().ct_eq(stored_hash.as_bytes()).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_hash_matches_only_its_own_secret() {
        let secret = LinkRequestSecret::mint();
        let stored = secret.hash();

        assert_eq!(stored.len(), 64);
        assert_ne!(stored, secret.as_str());
        assert!(secret.matches(&stored));
        assert!(!LinkRequestSecret::mint().matches(&stored));
        assert!(LinkRequestSecret::from_url(secret.as_str().to_string())
            .matches(&stored));
    }
}
