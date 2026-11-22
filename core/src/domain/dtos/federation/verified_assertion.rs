use super::ProfileAssertion;

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

/// A peer's assertion whose signature, audience, subject and lifetime were
/// checked (FED-14)
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifiedAssertion {
    /// The compact JWS as received, kept for audit
    pub jws: String,
    pub payload: ProfileAssertion,
    pub issued_at: DateTime<Local>,
    pub expires_at: DateTime<Local>,
}

impl VerifiedAssertion {
    pub fn is_valid_at(&self, now: DateTime<Local>) -> bool {
        now < self.expires_at
    }

    /// The same assertion, kept no later than `latest` even when the issuer
    /// signed a longer lifetime: the short TTL is the revocation backstop
    /// (D-1), so a peer cannot make its grants outlive it here
    pub fn kept_until(mut self, latest: DateTime<Local>) -> Self {
        self.expires_at = self.expires_at.min(latest);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn a_long_lifetime_is_cut_to_the_local_bound_and_a_short_one_kept() {
        let now = Local::now();
        let assertion = |lifetime| VerifiedAssertion {
            jws: "jws".to_string(),
            payload: ProfileAssertion::default(),
            issued_at: now,
            expires_at: now + lifetime,
        };
        let bound = now + Duration::seconds(60);

        let long = assertion(Duration::days(30)).kept_until(bound);
        let short = assertion(Duration::seconds(10)).kept_until(bound);

        assert_eq!(long.expires_at, bound);
        assert_eq!(short.expires_at, now + Duration::seconds(10));
    }
}
