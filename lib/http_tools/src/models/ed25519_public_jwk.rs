use crate::models::invalid_verification_setting;

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use myc_core::domain::dtos::profile_signature::SigningKeyVersion;
use mycelium_base::utils::errors::MappedErrors;
use serde_json::{json, Map, Value};
use std::str::FromStr;

/// Length of a raw Ed25519 public key (RFC 8032)
pub const ED25519_PUBLIC_KEY_BYTES: usize = 32;

/// Label used in configuration errors about a published key set
const JWKS: &str = "profile JWKS";

/// One profile-signing public key as a JSON Web Key (RFC 7517 / RFC 8037)
///
/// Shared by the gateway, which publishes it, and by the Rust downstream
/// verifier, which reads it back. Reading is strict: a private member (`d`),
/// another key type or curve, a key that is not 32 bytes, a non-canonical
/// `kid` or a non-signature use is a configuration error (`MYC00035`), never
/// a silently skipped key.
///
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ed25519PublicJwk {
    version: SigningKeyVersion,
    public_key: [u8; ED25519_PUBLIC_KEY_BYTES],
}

impl Ed25519PublicJwk {
    pub fn new(
        version: SigningKeyVersion,
        public_key: [u8; ED25519_PUBLIC_KEY_BYTES],
    ) -> Self {
        Self {
            version,
            public_key,
        }
    }

    pub fn version(&self) -> SigningKeyVersion {
        self.version
    }

    /// The `x` member: the raw public key, Base64URL without padding
    pub fn x(&self) -> String {
        URL_SAFE_NO_PAD.encode(self.public_key)
    }

    /// The JWK published by the gateway (never `d`, never `x5c`)
    pub fn to_json(&self) -> Value {
        json!({
            "kty": "OKP",
            "crv": "Ed25519",
            "x": self.x(),
            "kid": self.version.to_string(),
            "use": "sig",
            "alg": "EdDSA",
        })
    }

    /// Read one JWK of a published profile key set
    pub fn from_json(value: &Value) -> Result<Self, MappedErrors> {
        let Value::Object(members) = value else {
            return Err(invalid("entry is not a JSON object"));
        };

        if members.contains_key("d") {
            return Err(invalid("entry carries private key material (d)"));
        }

        expect_member(members, "kty", "OKP", true)?;
        expect_member(members, "crv", "Ed25519", true)?;
        expect_member(members, "use", "sig", false)?;
        expect_member(members, "alg", "EdDSA", false)?;

        Ok(Self::new(read_kid(members)?, read_x(members)?))
    }
}

fn expect_member(
    members: &Map<String, Value>,
    name: &str,
    expected: &str,
    required: bool,
) -> Result<(), MappedErrors> {
    match members.get(name) {
        None if !required => Ok(()),
        Some(Value::String(value)) if value == expected => Ok(()),
        _ => Err(invalid(&format!("entry member {name} must be {expected}"))),
    }
}

fn read_kid(
    members: &Map<String, Value>,
) -> Result<SigningKeyVersion, MappedErrors> {
    members
        .get("kid")
        .and_then(Value::as_str)
        .and_then(|kid| SigningKeyVersion::from_str(kid).ok())
        .ok_or_else(|| {
            invalid("entry kid must be a canonical positive integer")
        })
}

fn read_x(
    members: &Map<String, Value>,
) -> Result<[u8; ED25519_PUBLIC_KEY_BYTES], MappedErrors> {
    members
        .get("x")
        .and_then(Value::as_str)
        .and_then(|x| URL_SAFE_NO_PAD.decode(x).ok())
        .and_then(|bytes| bytes.try_into().ok())
        .ok_or_else(|| invalid("entry x must be a 32-byte Base64URL key"))
}

fn invalid(problem: &str) -> MappedErrors {
    invalid_verification_setting(JWKS, problem)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// TEST ONLY: RFC 8032 section 7.1 test vector 1 public key
    const X: &str = "11qYAYKxCrfVS_7TyWQHOg7hcvPapiMlrwIaaPcHURo";

    fn valid() -> Value {
        json!({"kty": "OKP", "crv": "Ed25519", "x": X, "kid": "1"})
    }

    fn with(member: &str, value: Value) -> Value {
        let mut jwk = valid();
        jwk[member] = value;
        jwk
    }

    #[test]
    fn round_trips_the_published_form() {
        let jwk = Ed25519PublicJwk::from_json(&valid()).unwrap();
        let published = jwk.to_json();

        assert_eq!(published["x"], X);
        assert_eq!(published["kid"], "1");
        assert_eq!(published["alg"], "EdDSA");
        assert!(published.get("d").is_none());
        assert_eq!(Ed25519PublicJwk::from_json(&published).unwrap(), jwk);
    }

    #[test]
    fn rejects_every_unsafe_or_foreign_entry() {
        let invalid = [
            json!("not an object"),
            with("d", json!(X)),
            with("kty", json!("RSA")),
            with("crv", json!("X25519")),
            with("x", json!("AAAA")),
            with("x", json!(format!("{X}=="))),
            with("kid", json!("01")),
            with("kid", json!(1)),
            with("use", json!("enc")),
            with("alg", json!("RS256")),
        ];

        for entry in invalid {
            let error = Ed25519PublicJwk::from_json(&entry).unwrap_err();
            assert!(error.has_str_code("MYC00035"), "{entry}");
        }
    }
}
