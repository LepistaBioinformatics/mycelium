use super::Ed25519PublicJwk;

use myc_core::domain::dtos::profile_signature::JwksMaxAgeSeconds;
use serde_json::{json, Value};
use std::sync::Arc;

/// The profile JWKS the gateway publishes to downstream verifiers
///
/// Built once at boot from every configured signing key (not only the
/// primary), so a downstream already knows the next key before the gateway
/// starts signing with it. The body is serialized once; the route only
/// returns it. `Disabled` when `[core.profileSigning]` is absent.
///
#[derive(Clone, Debug)]
pub enum PublishedProfileKeys {
    Disabled,
    Enabled {
        body: Arc<str>,
        max_age: JwksMaxAgeSeconds,
    },
}

impl PublishedProfileKeys {
    pub fn new(keys: &[Ed25519PublicJwk], max_age: JwksMaxAgeSeconds) -> Self {
        let mut sorted: Vec<&Ed25519PublicJwk> = keys.iter().collect();
        sorted.sort_by_key(|key| key.version());

        let document = json!({
            "keys": sorted.iter().map(|key| key.to_json()).collect::<Vec<Value>>(),
        });

        Self::Enabled {
            body: Arc::from(document.to_string()),
            max_age,
        }
    }

    /// Value of the `Cache-Control` response header
    pub fn cache_control(max_age: JwksMaxAgeSeconds) -> String {
        format!("public, max-age={}", max_age.value())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use myc_core::domain::dtos::profile_signature::SigningKeyVersion;

    fn key(version: u32, byte: u8) -> Ed25519PublicJwk {
        Ed25519PublicJwk::new(
            SigningKeyVersion::try_from(version).unwrap(),
            [byte; 32],
        )
    }

    #[test]
    fn publishes_every_key_sorted_by_version() {
        let published = PublishedProfileKeys::new(
            &[key(2, 2), key(1, 1)],
            JwksMaxAgeSeconds::default(),
        );

        let PublishedProfileKeys::Enabled { body, max_age } = published else {
            panic!("expected an enabled key set");
        };
        let document: Value = serde_json::from_str(&body).unwrap();
        let kids: Vec<&str> = document["keys"]
            .as_array()
            .unwrap()
            .iter()
            .map(|jwk| jwk["kid"].as_str().unwrap())
            .collect();

        assert_eq!(kids, ["1", "2"]);
        assert!(!body.contains("\"d\""));
        assert_eq!(
            PublishedProfileKeys::cache_control(max_age),
            "public, max-age=43200"
        );
    }
}
