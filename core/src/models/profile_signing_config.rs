use super::ProfileSigningKeySet;
use crate::domain::dtos::profile_signature::SigningKeyVersion;

use mycelium_base::utils::errors::MappedErrors;
use serde::Deserialize;

/// Downstream profile signing configuration (`[core.profileSigning]`)
///
/// Optional: when the section is absent the gateway does not sign and keeps
/// forwarding `x-mycelium-profile` exactly as before. When present, it must
/// be valid or the gateway refuses to start.
///
/// ```toml
/// [core.profileSigning]
/// primaryKeyVersion = 1
///
/// [[core.profileSigning.keys]]
/// version = 1
/// privateKey = { env = "MYC_PROFILE_PRIVATE_KEY_1" }
/// ```
///
/// `validate` only checks structure. Key material is resolved and imported
/// asynchronously at boot, after Vault is ready.
///
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileSigningConfig {
    pub primary_key_version: SigningKeyVersion,
    pub keys: ProfileSigningKeySet,
}

impl ProfileSigningConfig {
    pub fn validate(&self) -> Result<(), MappedErrors> {
        self.keys.validate(self.primary_key_version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Wrapper {
        profile_signing: ProfileSigningConfig,
    }

    fn parse(raw: &str) -> Result<ProfileSigningConfig, toml::de::Error> {
        toml::from_str::<Wrapper>(raw).map(|wrapper| wrapper.profile_signing)
    }

    #[test]
    fn parses_env_and_vault_entries() {
        let config = parse(
            r#"
            [profileSigning]
            primaryKeyVersion = 2

            [[profileSigning.keys]]
            version = 1
            privateKey = { env = "MYC_PROFILE_PRIVATE_KEY_1" }

            [[profileSigning.keys]]
            version = 2
            privateKey = { vault = { path = "myc/core/profileSigning", key = "v2" } }
            "#,
        )
        .unwrap();

        config.validate().unwrap();
        assert_eq!(config.primary_key_version.value(), 2);
        assert_eq!(config.keys.len(), 2);
    }

    #[test]
    fn rejects_incomplete_section() {
        assert!(parse("[profileSigning]\nprimaryKeyVersion = 1\n").is_err());
    }

    #[test]
    fn rejects_zero_version_at_parse_time() {
        let raw = r#"
            [profileSigning]
            primaryKeyVersion = 0

            [[profileSigning.keys]]
            version = 0
            privateKey = { env = "X" }
        "#;

        assert!(parse(raw).is_err());
    }

    #[test]
    fn validate_rejects_unknown_primary() {
        let config = parse(
            r#"
            [profileSigning]
            primaryKeyVersion = 9

            [[profileSigning.keys]]
            version = 1
            privateKey = { env = "X" }
            "#,
        )
        .unwrap();

        assert!(config.validate().is_err());
    }
}
