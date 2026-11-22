use myc_core::{
    domain::{
        dtos::{
            native_error_codes::NativeErrorCodes,
            profile_signature::SigningKeyVersion,
        },
        entities::InstanceSettingsRegistration,
    },
    models::{AccountLifeCycle, FederationConfig},
    use_cases::federation::get_or_mint_instance_id,
};
use myc_http_tools::models::{
    Ed25519PublicJwk, KeyDerivationPurpose, VersionedEd25519Signer,
};
use mycelium_base::utils::errors::{execution_err, MappedErrors};
use std::{sync::Arc, time::Duration};
use tokio::time::timeout;
use uuid::Uuid;

/// Upper bound for resolving every HMAC secret (env or Vault), as for the
/// profile signing keys
const RESOLUTION_BUDGET: Duration = Duration::from_secs(60);

/// The federation signing keys: the primary signer and every public key
#[derive(Clone, Debug)]
pub(crate) struct FederationSigning {
    // Read once peer requests are signed (federation tasks T17/T18); drop the
    // allow then.
    #[allow(dead_code)]
    pub(crate) primary: Arc<VersionedEd25519Signer>,
    pub(crate) public_keys: Vec<Ed25519PublicJwk>,
}

/// What boot preparation of federation produces when `[core.federation]` is
/// present
#[derive(Clone, Debug)]
pub(crate) struct PreparedFederation {
    pub(crate) config: FederationConfig,
    pub(crate) instance_id: Uuid,
    pub(crate) signing: FederationSigning,
}

/// Prepare federation once, at boot
///
/// Nothing happens without `[core.federation]`. With it, one Ed25519 key is
/// derived from every `hmacSecrets` entry under the federation label (D-8),
/// the key of `hmacPrimaryVersion` signs, and the instance id is minted or
/// read back (FED-01). Any failure refuses the boot (FED-02). The private
/// keys only ever exist in memory (FED-05).
///
#[tracing::instrument(name = "prepare_federation", skip_all)]
pub(crate) async fn prepare_federation(
    config: Option<&FederationConfig>,
    life_cycle: &AccountLifeCycle,
    instance_settings_registration_repo: &dyn InstanceSettingsRegistration,
) -> Result<Option<PreparedFederation>, MappedErrors> {
    let Some(config) = config else {
        return Ok(None);
    };

    let primary = life_cycle.hmac_primary_version();
    let secrets = timeout(RESOLUTION_BUDGET, life_cycle.hmac_signing_keys())
        .await
        .map_err(|_| invalid(primary, "resolution_timeout"))?
        .map_err(|_| invalid(primary, "resolution_failed"))?;

    let signing = derive_federation_signing(&secrets, primary)?;
    let instance_id =
        get_or_mint_instance_id(Box::new(instance_settings_registration_repo))
            .await?;

    tracing::info!(
        event = "federation.startup",
        instance_id = %instance_id,
        primary_key_version = primary,
        key_versions = signing.public_keys.len(),
    );

    Ok(Some(PreparedFederation {
        config: config.clone(),
        instance_id,
        signing,
    }))
}

/// Derive the federation signer and public keys from the resolved secrets
///
/// Version `0` is never a `kid`. Unlike the profile signature, a primary
/// without a key is an error, not a disabled state: a peer cannot be served
/// without a signing key.
///
fn derive_federation_signing(
    secrets: &[(u32, Vec<u8>)],
    primary: u32,
) -> Result<FederationSigning, MappedErrors> {
    let mut signers = Vec::with_capacity(secrets.len());
    for (raw_version, secret) in secrets {
        let Ok(version) = SigningKeyVersion::try_from(*raw_version) else {
            continue;
        };

        signers.push(VersionedEd25519Signer::derive_from_secret(
            version,
            secret,
            KeyDerivationPurpose::Federation,
        )?);
    }

    let public_keys =
        signers.iter().map(|one| one.public_jwk().clone()).collect();

    let primary_signer = signers
        .into_iter()
        .find(|signer| signer.version().value() == primary)
        .ok_or_else(|| invalid(primary, "primary_without_key"))?;

    Ok(FederationSigning {
        primary: Arc::new(primary_signer),
        public_keys,
    })
}

fn invalid(version: u32, step: &str) -> MappedErrors {
    execution_err(format!(
        "federation_signing_key_invalid: version {version}, step {step}"
    ))
    .with_code(NativeErrorCodes::MYC00041)
}

#[cfg(test)]
mod tests {
    use super::*;
    use myc_core::domain::dtos::profile_signature::SigningKeyVersion;
    use myc_http_tools::models::KeyDerivationPurpose;

    fn secrets(versions: &[u32]) -> Vec<(u32, Vec<u8>)> {
        versions
            .iter()
            .map(|version| {
                (*version, format!("secret-v{version}").into_bytes())
            })
            .collect()
    }

    fn kids(signing: &FederationSigning) -> Vec<u32> {
        signing
            .public_keys
            .iter()
            .map(|key| key.version().value())
            .collect()
    }

    #[test]
    fn one_key_per_version_with_the_version_as_kid() {
        let signing =
            derive_federation_signing(&secrets(&[1, 2, 3]), 2).unwrap();

        assert_eq!(kids(&signing), [1, 2, 3]);
    }

    #[test]
    fn the_primary_version_signs() {
        let signing = derive_federation_signing(&secrets(&[1, 2]), 2).unwrap();

        assert_eq!(signing.primary.version().value(), 2);
        assert_eq!(signing.primary.public_jwk(), &signing.public_keys[1]);
    }

    #[test]
    fn version_zero_never_becomes_a_kid() {
        let signing = derive_federation_signing(&secrets(&[0, 1]), 1).unwrap();

        assert_eq!(kids(&signing), [1]);
    }

    #[test]
    fn a_primary_without_a_key_refuses() {
        assert!(derive_federation_signing(&secrets(&[0]), 0).is_err());
        assert!(derive_federation_signing(&secrets(&[1]), 2).is_err());
    }

    #[test]
    fn the_federation_key_differs_from_the_profile_key() {
        let signing = derive_federation_signing(&secrets(&[1]), 1).unwrap();
        let profile = VersionedEd25519Signer::derive_from_secret(
            SigningKeyVersion::try_from(1).unwrap(),
            b"secret-v1",
            KeyDerivationPurpose::ProfileSigning,
        )
        .unwrap();

        assert_ne!(signing.primary.public_jwk().x(), profile.public_jwk().x());
    }
}
