use super::{
    invalid_verification_setting, ProfileJwksFetcher, ProfileJwksUrl,
    ProfileKeyStore, ProfileSignatureTolerance, ProfileSignatureVerifier,
};
use crate::{
    functions::{read_signature_key_version, verify_and_decode_profile},
    settings::{
        PROFILE_JWKS_ALLOW_INSECURE_HTTP_ENV,
        PROFILE_JWKS_REFETCH_COOLDOWN_SECONDS, PROFILE_JWKS_URL_ENV,
        PROFILE_SIGNATURE_AUDIENCE_ENV, PROFILE_SIGNATURE_TOLERANCE_ENV,
    },
};

use myc_core::domain::dtos::{
    profile::Profile,
    profile_signature::{
        ProfileSignatureAudience, ProfileSignatureRejection,
        UnixTimestampSeconds,
    },
};
use mycelium_base::utils::errors::MappedErrors;
use std::{collections::BTreeMap, time::Duration};

/// Downstream verifier whose trusted keys come from the gateway JWKS
///
/// Enabled by `MYCELIUM_JWKS_URL`; `MYC_PROFILE_SIGNATURE_AUDIENCE` is then
/// required, so a signed pair issued for another service is always refused.
/// The verification itself is the shared, local contract; only the keys are
/// remote, refreshed as described in `ProfileKeyStore`.
///
#[derive(Debug)]
pub struct JwksProfileSignatureVerifier {
    store: ProfileKeyStore,
    tolerance: ProfileSignatureTolerance,
    audience: ProfileSignatureAudience,
}

impl JwksProfileSignatureVerifier {
    pub fn new(
        store: ProfileKeyStore,
        tolerance: ProfileSignatureTolerance,
        audience: ProfileSignatureAudience,
    ) -> Self {
        Self {
            store,
            tolerance,
            audience,
        }
    }

    /// Read the settings; `None` when `MYCELIUM_JWKS_URL` is absent
    ///
    /// Never touches the network. Every present setting is validated, even
    /// when verification stays disabled.
    ///
    pub fn from_variables(
        variables: &BTreeMap<String, String>,
        fetcher: ProfileJwksFetcher,
    ) -> Result<Option<Self>, MappedErrors> {
        let tolerance = ProfileSignatureTolerance::from_variable(
            variables.get(PROFILE_SIGNATURE_TOLERANCE_ENV),
        )?;
        let audience = parse_audience(variables)?;
        let allow_insecure_http = parse_insecure_switch(variables)?;

        let Some(raw_url) = variables.get(PROFILE_JWKS_URL_ENV) else {
            return Ok(None);
        };
        let url = ProfileJwksUrl::parse(raw_url, allow_insecure_http)?;
        let audience = audience.ok_or_else(missing_audience)?;

        let cooldown =
            Duration::from_secs(PROFILE_JWKS_REFETCH_COOLDOWN_SECONDS);
        let store = ProfileKeyStore::new(url, fetcher, cooldown);

        Ok(Some(Self::new(store, tolerance, audience)))
    }

    /// Download the key set before serving; failures abort startup
    #[tracing::instrument(name = "profile_jwks_prefetch", skip_all)]
    pub async fn prefetch(&self) -> Result<(), MappedErrors> {
        self.store.prefetch().await
    }

    /// Verify the signed profile, refetching once on an unknown `kid`
    #[tracing::instrument(name = "verify_profile_with_jwks", skip_all)]
    pub async fn verify_and_decode(
        &self,
        profile_header: &str,
        signature_header: &str,
        now: UnixTimestampSeconds,
    ) -> Result<Profile, ProfileSignatureRejection> {
        self.store.refresh_if_stale().await;
        let (first, generation) = self.verify_with_current_keys(
            profile_header,
            signature_header,
            now,
        );

        if !deserves_refetch(&first, signature_header)
            || !self.store.refresh_after_unknown_key(generation).await
        {
            return first;
        }

        self.verify_with_current_keys(profile_header, signature_header, now)
            .0
    }

    fn verify_with_current_keys(
        &self,
        profile_header: &str,
        signature_header: &str,
        now: UnixTimestampSeconds,
    ) -> (Result<Profile, ProfileSignatureRejection>, u64) {
        let (keys, generation) = self.store.current();
        let verifier = ProfileSignatureVerifier::new(
            keys,
            self.tolerance,
            Some(self.audience.clone()),
        );
        let outcome = verify_and_decode_profile(
            profile_header,
            signature_header,
            &verifier,
            now,
        );

        (outcome, generation)
    }
}

/// Only an unknown but canonical `kid` justifies network work
fn deserves_refetch(
    outcome: &Result<Profile, ProfileSignatureRejection>,
    signature_header: &str,
) -> bool {
    matches!(outcome, Err(ProfileSignatureRejection::UnknownKey))
        && read_signature_key_version(signature_header).is_some()
}

fn missing_audience() -> MappedErrors {
    invalid_verification_setting(
        PROFILE_SIGNATURE_AUDIENCE_ENV,
        "is required when MYCELIUM_JWKS_URL is set",
    )
}

fn parse_audience(
    variables: &BTreeMap<String, String>,
) -> Result<Option<ProfileSignatureAudience>, MappedErrors> {
    let Some(raw) = variables.get(PROFILE_SIGNATURE_AUDIENCE_ENV) else {
        return Ok(None);
    };

    ProfileSignatureAudience::try_from(raw.to_owned())
        .map(Some)
        .map_err(|_| {
            invalid_verification_setting(
                PROFILE_SIGNATURE_AUDIENCE_ENV,
                "must not be empty",
            )
        })
}

fn parse_insecure_switch(
    variables: &BTreeMap<String, String>,
) -> Result<bool, MappedErrors> {
    match variables
        .get(PROFILE_JWKS_ALLOW_INSECURE_HTTP_ENV)
        .map(|raw| raw.trim())
    {
        None | Some("false") => Ok(false),
        Some("true") => Ok(true),
        Some(_) => Err(invalid_verification_setting(
            PROFILE_JWKS_ALLOW_INSECURE_HTTP_ENV,
            "must be true or false",
        )),
    }
}
