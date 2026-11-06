use super::Ed25519PublicJwk;

use jsonwebtoken::DecodingKey;
use myc_core::domain::dtos::{
    native_error_codes::NativeErrorCodes, profile_signature::SigningKeyVersion,
};
use mycelium_base::utils::errors::{dto_err, MappedErrors};
use serde_json::Value;
use std::{collections::BTreeMap, str::FromStr};

/// Label used in configuration errors about a published key set
const JWKS: &str = "profile JWKS";

/// The Ed25519 public keys a downstream service trusts, by key version
///
/// Read from the gateway's profile JWKS. The document is accepted or refused
/// as a whole: one unsafe entry (private member, other key type or curve,
/// wrong length, non-canonical or duplicated `kid`) refuses the set, so the
/// verifier keeps its last valid one instead of trusting a partial set.
///
#[derive(Clone, Debug, Default)]
pub struct TrustedProfileKeys(BTreeMap<SigningKeyVersion, DecodingKey>);

impl TrustedProfileKeys {
    /// Parse a JWKS document (`{"keys": [...]}`)
    pub fn from_jwks_document(document: &[u8]) -> Result<Self, MappedErrors> {
        let parsed: Value = serde_json::from_slice(document)
            .map_err(|_| invalid_verification_setting(JWKS, "is not JSON"))?;

        let mut keys = BTreeMap::new();
        for entry in read_entries(&parsed)? {
            insert_unique(&mut keys, Ed25519PublicJwk::from_json(entry)?)?;
        }

        Ok(Self(keys))
    }

    /// The key selected by a JWS `kid`, when the version is trusted
    pub fn key_for(&self, kid: &str) -> Option<&DecodingKey> {
        let version = SigningKeyVersion::from_str(kid).ok()?;

        self.0.get(&version)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

fn read_entries(parsed: &Value) -> Result<&Vec<Value>, MappedErrors> {
    parsed
        .get("keys")
        .and_then(Value::as_array)
        .filter(|entries| !entries.is_empty())
        .ok_or_else(|| invalid_verification_setting(JWKS, "has no keys array"))
}

fn insert_unique(
    keys: &mut BTreeMap<SigningKeyVersion, DecodingKey>,
    jwk: Ed25519PublicJwk,
) -> Result<(), MappedErrors> {
    let key = decoding_key(&jwk)?;

    match keys.insert(jwk.version(), key) {
        None => Ok(()),
        Some(_) => {
            Err(invalid_verification_setting(JWKS, "has a duplicated kid"))
        }
    }
}

fn decoding_key(jwk: &Ed25519PublicJwk) -> Result<DecodingKey, MappedErrors> {
    DecodingKey::from_ed_components(&jwk.x()).map_err(|_| {
        invalid_verification_setting(JWKS, "entry x is not an Ed25519 key")
    })
}

/// Configuration error of a downstream verification setting (`MYC00035`)
///
/// Names the setting and the problem only; never echoes the value.
///
pub(crate) fn invalid_verification_setting(
    name: &str,
    problem: &str,
) -> MappedErrors {
    dto_err(format!("{name} {problem}")).with_code(NativeErrorCodes::MYC00035)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// TEST ONLY: RFC 8032 section 7.1 test vectors 1 and 2 public keys
    const X1: &str = "11qYAYKxCrfVS_7TyWQHOg7hcvPapiMlrwIaaPcHURo";
    const X2: &str = "PUAXw-hDiVqStwqnTRt-vJyYLM8uxJaMwM1V8Sr0Zgw";

    fn jwk(kid: &str, x: &str) -> Value {
        json!({"kty": "OKP", "crv": "Ed25519", "x": x, "kid": kid})
    }

    fn parse(document: Value) -> Result<TrustedProfileKeys, MappedErrors> {
        TrustedProfileKeys::from_jwks_document(document.to_string().as_bytes())
    }

    #[test]
    fn selects_keys_by_canonical_kid_only() {
        let keys =
            parse(json!({"keys": [jwk("1", X1), jwk("2", X2)]})).unwrap();

        assert_eq!(keys.len(), 2);
        assert!(keys.key_for("1").is_some());
        assert!(keys.key_for("2").is_some());
        assert!(keys.key_for("3").is_none());
        assert!(keys.key_for("01").is_none());
        assert!(keys.key_for("../1").is_none());
    }

    #[test]
    fn one_bad_entry_refuses_the_whole_set() {
        let mut private = jwk("2", X2);
        private["d"] = json!(X1);

        let invalid = [
            json!([]),
            json!({"keys": []}),
            json!({"keys": "x"}),
            json!({"keys": [jwk("1", X1), jwk("1", X2)]}),
            json!({"keys": [jwk("1", X1), private]}),
            json!({"keys": [jwk("1", X1), jwk("0", X2)]}),
        ];

        for document in invalid {
            let error = parse(document.clone()).unwrap_err();
            assert!(error.has_str_code("MYC00035"), "{document}");
        }

        let error =
            TrustedProfileKeys::from_jwks_document(b"not json").unwrap_err();
        assert!(error.has_str_code("MYC00035"));
    }
}
