use crate::settings::PROFILE_PUBLIC_KEY_ENV_PREFIX;

use jsonwebtoken::DecodingKey;
use myc_core::domain::dtos::{
    native_error_codes::NativeErrorCodes, profile_signature::SigningKeyVersion,
};
use mycelium_base::utils::errors::{dto_err, MappedErrors};
use std::{collections::BTreeMap, str::FromStr};

/// Marker of an SPKI public key PEM (a PKCS#8 private key is refused)
const PUBLIC_KEY_PEM_HEADER: &str = "-----BEGIN PUBLIC KEY-----";

/// The Ed25519 public keys a downstream service trusts, by key version
///
/// Built from the `MYC_PROFILE_PUBLIC_KEY_<version>` variables. The version
/// suffix must be canonical, since it is matched against the JWS `kid`, and
/// every value must be an Ed25519 SPKI PEM: a private key or a malformed PEM
/// is a configuration error, never a silently ignored key.
///
#[derive(Clone, Debug)]
pub struct TrustedProfileKeys(BTreeMap<SigningKeyVersion, DecodingKey>);

impl TrustedProfileKeys {
    /// Collect the trusted keys from environment-like variables
    ///
    /// Returns `None` when no variable carries the prefix: verification is
    /// disabled and the legacy behavior is preserved.
    ///
    pub fn from_variables(
        variables: &BTreeMap<String, String>,
    ) -> Result<Option<Self>, MappedErrors> {
        let mut keys = BTreeMap::new();

        for (name, pem) in variables.iter() {
            let Some(suffix) = name.strip_prefix(PROFILE_PUBLIC_KEY_ENV_PREFIX)
            else {
                continue;
            };

            keys.insert(parse_version(name, suffix)?, parse_key(name, pem)?);
        }

        Ok((!keys.is_empty()).then_some(Self(keys)))
    }

    /// The key selected by a JWS `kid`, when the version is trusted
    pub fn key_for(&self, kid: &str) -> Option<&DecodingKey> {
        let version = SigningKeyVersion::from_str(kid).ok()?;

        self.0.get(&version)
    }
}

fn parse_version(
    name: &str,
    suffix: &str,
) -> Result<SigningKeyVersion, MappedErrors> {
    SigningKeyVersion::from_str(suffix).map_err(|_| {
        invalid_verification_setting(
            name,
            "key version suffix must be a canonical positive integer",
        )
    })
}

fn parse_key(name: &str, pem: &str) -> Result<DecodingKey, MappedErrors> {
    if !pem.contains(PUBLIC_KEY_PEM_HEADER) {
        return Err(invalid_verification_setting(
            name,
            "is not a PEM SubjectPublicKeyInfo public key",
        ));
    }

    DecodingKey::from_ed_pem(pem.as_bytes()).map_err(|_| {
        invalid_verification_setting(name, "is not an Ed25519 public key")
    })
}

/// Configuration error of a downstream verification setting (`MYC00035`)
///
/// Names the variable and the problem only; never echoes the value.
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

    /// TEST ONLY: RFC 8032 test vector 1 (SPKI / PKCS#8)
    const PUBLIC_PEM: &str = "-----BEGIN PUBLIC KEY-----
MCowBQYDK2VwAyEA11qYAYKxCrfVS/7TyWQHOg7hcvPapiMlrwIaaPcHURo=
-----END PUBLIC KEY-----
";
    const PRIVATE_PEM: &str = "-----BEGIN PRIVATE KEY-----
MC4CAQAwBQYDK2VwBCIEIJ1hsZ3v/VpguoRK9JLsLMREScVpezJpGXA7rAMcrn9g
-----END PRIVATE KEY-----
";

    fn variables(entries: &[(&str, &str)]) -> BTreeMap<String, String> {
        entries
            .iter()
            .map(|(name, value)| (name.to_string(), value.to_string()))
            .collect()
    }

    #[test]
    fn no_prefixed_variable_disables_verification() {
        let keys =
            TrustedProfileKeys::from_variables(&variables(&[("PATH", "x")]))
                .unwrap();

        assert!(keys.is_none());
    }

    #[test]
    fn selects_keys_by_canonical_kid_only() {
        let keys = TrustedProfileKeys::from_variables(&variables(&[(
            "MYC_PROFILE_PUBLIC_KEY_1",
            PUBLIC_PEM,
        )]))
        .unwrap()
        .unwrap();

        assert!(keys.key_for("1").is_some());
        assert!(keys.key_for("2").is_none());
        assert!(keys.key_for("01").is_none());
        assert!(keys.key_for("../1").is_none());
    }

    #[test]
    fn invalid_entries_are_configuration_errors() {
        let invalid = [
            ("MYC_PROFILE_PUBLIC_KEY_0", PUBLIC_PEM),
            ("MYC_PROFILE_PUBLIC_KEY_01", PUBLIC_PEM),
            ("MYC_PROFILE_PUBLIC_KEY_v1", PUBLIC_PEM),
            ("MYC_PROFILE_PUBLIC_KEY_1", PRIVATE_PEM),
            ("MYC_PROFILE_PUBLIC_KEY_1", ""),
            (
                "MYC_PROFILE_PUBLIC_KEY_1",
                "-----BEGIN PUBLIC KEY-----\nAA==",
            ),
        ];

        for entry in invalid {
            let error =
                TrustedProfileKeys::from_variables(&variables(&[entry]))
                    .unwrap_err();

            assert!(error.msg().contains(entry.0));
            assert!(error.has_str_code("MYC00035"));
        }
    }
}
