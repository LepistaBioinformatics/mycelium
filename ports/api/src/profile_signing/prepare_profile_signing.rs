use super::PreparedProfileSigning;
use myc_core::{
    domain::dtos::{
        native_error_codes::NativeErrorCodes,
        profile_signature::{JwksMaxAgeSeconds, SigningKeyVersion},
    },
    models::AccountLifeCycle,
};
use myc_http_tools::models::{
    Ed25519PublicJwk, ProfileSigningState, PublishedProfileKeys,
    VersionedEd25519Signer,
};
use mycelium_base::utils::errors::{execution_err, MappedErrors};
use std::{sync::Arc, time::Duration};
use tokio::time::timeout;

/// Upper bound for resolving every HMAC secret (env or Vault)
///
/// The Vault resolver uses an HTTP client without its own timeout; boot must
/// fail instead of hanging.
///
const RESOLUTION_BUDGET: Duration = Duration::from_secs(60);

/// Shortest secret that does not weaken the derived Ed25519 key
const RECOMMENDED_SECRET_BYTES: usize = 32;

/// Prepare the downstream profile signing state once, at boot
///
/// No configuration of its own: one Ed25519 key is derived (HKDF-SHA256)
/// from every connection-string HMAC secret in
/// `core.accountLifeCycle.hmacSecrets`, keeping its version as the `kid`.
/// The gateway signs with the key of `hmacPrimaryVersion` and publishes the
/// public half of every version as the profile JWKS, so both features share
/// one rotation procedure and every replica derives the same keys.
///
/// Must run after Vault initialization and before the server accepts
/// traffic. Errors are sanitized: version and failing step, never material.
///
#[tracing::instrument(name = "prepare_profile_signing", skip_all)]
pub(crate) async fn prepare_profile_signing(
    life_cycle: &AccountLifeCycle,
) -> Result<PreparedProfileSigning, MappedErrors> {
    let primary = life_cycle.hmac_primary_version();
    let secrets = timeout(RESOLUTION_BUDGET, life_cycle.hmac_signing_keys())
        .await
        .map_err(|_| invalid(primary, "resolution_timeout"))?
        .map_err(|_| invalid(primary, "resolution_failed"))?;

    derive_profile_signing(&secrets, primary)
}

/// Derive the signer and the published keys from the resolved HMAC secrets
///
/// Version `0` cannot be a `kid` (reserved by the contract): such an entry is
/// not published, and when it is the primary the gateway does not sign.
///
fn derive_profile_signing(
    secrets: &[(u32, Vec<u8>)],
    primary: u32,
) -> Result<PreparedProfileSigning, MappedErrors> {
    let mut signers = Vec::with_capacity(secrets.len());
    for (version, secret) in secrets {
        signers.extend(derive_signer(*version, secret)?);
    }

    let public_keys: Vec<Ed25519PublicJwk> =
        signers.iter().map(|one| one.public_jwk().clone()).collect();
    let primary_signer = signers
        .into_iter()
        .find(|signer| signer.version().value() == primary);

    let Some(signer) = primary_signer else {
        announce_disabled(primary);
        return Ok(PreparedProfileSigning::disabled());
    };

    announce_enabled(primary, public_keys.len());
    Ok(PreparedProfileSigning {
        state: ProfileSigningState::Enabled(Arc::new(signer)),
        published_keys: PublishedProfileKeys::new(
            &public_keys,
            JwksMaxAgeSeconds::default(),
        ),
    })
}

/// The signer of one HMAC version; `None` for the reserved version `0`
fn derive_signer(
    raw_version: u32,
    secret: &[u8],
) -> Result<Option<VersionedEd25519Signer>, MappedErrors> {
    let Ok(version) = SigningKeyVersion::try_from(raw_version) else {
        return Ok(None);
    };

    warn_when_secret_is_short(version, secret);
    VersionedEd25519Signer::derive_from_secret(version, secret).map(Some)
}

fn announce_disabled(primary: u32) {
    tracing::warn!(
        event = "profile_signature.startup",
        enabled = false,
        primary_key_version = primary,
        "hmacPrimaryVersion cannot be a profile signature kid; \
         downstream profiles are not signed"
    );
}

fn announce_enabled(primary: u32, key_versions: usize) {
    tracing::info!(
        event = "profile_signature.startup",
        enabled = true,
        primary_key_version = primary,
        key_versions = key_versions,
    );
}

fn warn_when_secret_is_short(version: SigningKeyVersion, secret: &[u8]) {
    if secret.len() >= RECOMMENDED_SECRET_BYTES {
        return;
    }

    tracing::warn!(
        event = "profile_signature.startup",
        key_version = version.value(),
        "hmacSecrets entry is shorter than 32 bytes; the derived profile \
         signing key is only as strong as this secret"
    );
}

fn invalid(version: u32, step: &str) -> MappedErrors {
    execution_err(format!(
        "profile_signing_key_invalid: version {version}, step {step}"
    ))
    .with_code(NativeErrorCodes::MYC00041)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn secrets(versions: &[u32]) -> Vec<(u32, Vec<u8>)> {
        versions
            .iter()
            .map(|version| {
                (*version, format!("secret-v{version}").into_bytes())
            })
            .collect()
    }

    fn published_kids(prepared: &PreparedProfileSigning) -> Vec<String> {
        let PublishedProfileKeys::Enabled { body, .. } =
            &prepared.published_keys
        else {
            return vec![];
        };
        let document: Value = serde_json::from_str(body).unwrap();

        document["keys"]
            .as_array()
            .unwrap()
            .iter()
            .map(|key| key["kid"].as_str().unwrap().to_string())
            .collect()
    }

    fn signer_of(prepared: &PreparedProfileSigning) -> &VersionedEd25519Signer {
        let ProfileSigningState::Enabled(signer) = &prepared.state else {
            panic!("signing must be enabled");
        };

        signer
    }

    #[test]
    fn signs_with_the_primary_and_publishes_every_version() {
        let prepared = derive_profile_signing(&secrets(&[1, 2]), 2).unwrap();

        assert_eq!(signer_of(&prepared).version().value(), 2);
        assert_eq!(published_kids(&prepared), ["1", "2"]);
    }

    #[test]
    fn replicas_with_the_same_secrets_derive_the_same_keys() {
        let first = derive_profile_signing(&secrets(&[1, 2]), 1).unwrap();
        let second = derive_profile_signing(&secrets(&[1, 2]), 1).unwrap();

        assert_eq!(
            signer_of(&first).public_jwk(),
            signer_of(&second).public_jwk()
        );
    }

    #[test]
    fn each_version_derives_a_distinct_key() {
        let first = derive_profile_signing(&secrets(&[1]), 1).unwrap();
        let second = derive_profile_signing(&secrets(&[2]), 2).unwrap();

        assert_ne!(
            signer_of(&first).public_jwk().x(),
            signer_of(&second).public_jwk().x()
        );
    }

    #[test]
    fn version_zero_is_never_published() {
        let prepared = derive_profile_signing(&secrets(&[0, 1]), 1).unwrap();

        assert_eq!(published_kids(&prepared), ["1"]);
    }

    #[test]
    fn primary_version_zero_disables_signing() {
        let prepared = derive_profile_signing(&secrets(&[0]), 0).unwrap();

        assert!(!prepared.state.is_enabled());
        assert!(published_kids(&prepared).is_empty());
    }
}
