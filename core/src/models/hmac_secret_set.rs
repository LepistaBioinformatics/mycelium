use myc_config::secret_resolver::SecretResolver;
use mycelium_base::utils::errors::{dto_err, MappedErrors};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// A single HMAC key identified by its rotation version.
///
/// The version is embedded in every newly-signed connection string (via
/// the `KVR` bean) so verification can locate the right key at request
/// time — including for signatures issued under a previous primary.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct HmacSecretEntry {
    pub version: u32,
    pub secret: SecretResolver<String>,
}

/// Versioned set of HMAC keys available for connection-string signing
/// and verification.
///
/// First-class collection (object-calisthenics rule) — owns the
/// invariant that versions are unique and that lookups go through
/// `lookup(version)`. The primary write version lives on
/// `AccountLifeCycle::hmac_primary_version` and is cross-validated
/// against this set at startup.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(transparent)]
pub struct HmacSecretSet(Vec<HmacSecretEntry>);

impl HmacSecretSet {
    pub fn new(entries: Vec<HmacSecretEntry>) -> Self {
        Self(entries)
    }

    /// Look up the entry for `version`, if any.
    pub fn lookup(&self, version: u32) -> Option<&HmacSecretEntry> {
        self.0.iter().find(|entry| entry.version == version)
    }

    /// Validate the set against a declared primary version.
    ///
    /// Fails if the set is empty, if any two entries share a version, or
    /// if `primary` is not present in the set.
    pub fn validate(&self, primary: u32) -> Result<(), MappedErrors> {
        if self.0.is_empty() {
            return dto_err("hmac_secrets_set_is_empty").as_error();
        }

        let mut seen: HashSet<u32> = HashSet::with_capacity(self.0.len());
        for entry in &self.0 {
            if !seen.insert(entry.version) {
                return dto_err(format!(
                    "hmac_secrets_duplicate_version: {}",
                    entry.version,
                ))
                .as_error();
            }
        }

        if !seen.contains(&primary) {
            return dto_err(format!(
                "hmac_primary_version_not_in_set: {primary}",
            ))
            .as_error();
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(version: u32, value: &str) -> HmacSecretEntry {
        HmacSecretEntry {
            version,
            secret: SecretResolver::Value(value.to_string()),
        }
    }

    #[test]
    fn validate_rejects_missing_primary() {
        let set = HmacSecretSet::new(vec![entry(1, "k1")]);
        let outcome = set.validate(2);
        assert!(outcome.is_err());
    }

    #[test]
    fn validate_rejects_duplicate_versions() {
        let set = HmacSecretSet::new(vec![entry(1, "k1"), entry(1, "k1-bis")]);
        let outcome = set.validate(1);
        assert!(outcome.is_err());
    }

    #[test]
    fn validate_accepts_minimal_valid_set() {
        let set = HmacSecretSet::new(vec![entry(1, "k1")]);
        set.validate(1).expect("single-entry set is valid");
    }

    #[test]
    fn lookup_returns_some() {
        let set = HmacSecretSet::new(vec![entry(1, "k1"), entry(2, "k2")]);
        let found = set.lookup(2);
        assert!(found.is_some());
        assert_eq!(found.unwrap().version, 2);
    }

    #[test]
    fn lookup_returns_none_for_unknown() {
        let set = HmacSecretSet::new(vec![entry(1, "k1")]);
        assert!(set.lookup(99).is_none());
    }
}
