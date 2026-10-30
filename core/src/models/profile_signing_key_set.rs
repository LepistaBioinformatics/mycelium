use crate::domain::dtos::{
    native_error_codes::NativeErrorCodes, profile_signature::SigningKeyVersion,
};

use myc_config::secret_resolver::SecretResolver;
use mycelium_base::utils::errors::{dto_err, MappedErrors};
use serde::Deserialize;
use std::{
    collections::HashSet,
    fmt::{Debug, Formatter, Result as FmtResult},
};

/// One Ed25519 private key used to sign downstream profiles
///
/// The PEM (PKCS#8) is resolved through `SecretResolver` at boot, after Vault
/// is initialized. `Debug` never prints the resolver: a `Value` variant would
/// carry the private key itself.
///
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileSigningKeyEntry {
    pub version: SigningKeyVersion,
    pub private_key: SecretResolver<String>,
}

impl ProfileSigningKeyEntry {
    /// Name of the resolver source, safe to log
    pub fn source_kind(&self) -> &'static str {
        match self.private_key {
            SecretResolver::Env(_) => "env",
            SecretResolver::Vault { .. } => "vault",
            SecretResolver::Value(_) => "value",
        }
    }

    pub fn uses_vault(&self) -> bool {
        matches!(self.private_key, SecretResolver::Vault { .. })
    }
}

impl Debug for ProfileSigningKeyEntry {
    fn fmt(&self, formatter: &mut Formatter) -> FmtResult {
        formatter
            .debug_struct("ProfileSigningKeyEntry")
            .field("version", &self.version)
            .field("private_key", &format!("<{}:redacted>", self.source_kind()))
            .finish()
    }
}

/// Versioned set of downstream profile signing keys
///
/// First-class collection: owns the invariants that the set is not empty,
/// versions are unique and the declared primary is listed. Deliberately
/// separate from `HmacSecretSet`: different purpose, rotation cadence and
/// key material.
///
#[derive(Clone, Debug, Deserialize)]
#[serde(transparent)]
pub struct ProfileSigningKeySet(Vec<ProfileSigningKeyEntry>);

impl ProfileSigningKeySet {
    pub fn new(entries: Vec<ProfileSigningKeyEntry>) -> Self {
        Self(entries)
    }

    pub fn iter(&self) -> impl Iterator<Item = &ProfileSigningKeyEntry> {
        self.0.iter()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn validate(
        &self,
        primary: SigningKeyVersion,
    ) -> Result<(), MappedErrors> {
        if self.0.is_empty() {
            return invalid("profile_signing_keys_set_is_empty".to_string());
        }

        let mut seen = HashSet::with_capacity(self.0.len());
        let duplicated = self.0.iter().find(|key| !seen.insert(key.version));

        if let Some(entry) = duplicated {
            return invalid(format!(
                "profile_signing_keys_duplicate_version: {}",
                entry.version
            ));
        }

        if !seen.contains(&primary) {
            return invalid(format!(
                "profile_signing_primary_version_not_in_set: {primary}"
            ));
        }

        Ok(())
    }
}

fn invalid(message: String) -> Result<(), MappedErrors> {
    dto_err(message)
        .with_code(NativeErrorCodes::MYC00033)
        .as_error()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn version(value: u32) -> SigningKeyVersion {
        SigningKeyVersion::try_from(value).unwrap()
    }

    fn entry(value: u32) -> ProfileSigningKeyEntry {
        ProfileSigningKeyEntry {
            version: version(value),
            private_key: SecretResolver::Value(
                "-----BEGIN PRIVATE KEY-----secret".to_string(),
            ),
        }
    }

    #[test]
    fn rejects_empty_set() {
        let set = ProfileSigningKeySet::new(vec![]);
        assert!(set.validate(version(1)).is_err());
    }

    #[test]
    fn rejects_duplicated_versions() {
        let set = ProfileSigningKeySet::new(vec![entry(1), entry(1)]);
        assert!(set.validate(version(1)).is_err());
    }

    #[test]
    fn rejects_missing_primary() {
        let set = ProfileSigningKeySet::new(vec![entry(1)]);
        assert!(set.validate(version(2)).is_err());
    }

    #[test]
    fn accepts_rotation_set() {
        let set = ProfileSigningKeySet::new(vec![entry(1), entry(2)]);
        set.validate(version(2)).unwrap();
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn debug_never_prints_key_material() {
        let rendered =
            format!("{:?}", ProfileSigningKeySet::new(vec![entry(1)]));

        assert!(!rendered.contains("BEGIN"));
        assert!(!rendered.contains("secret"));
        assert!(rendered.contains("<value:redacted>"));
    }
}
