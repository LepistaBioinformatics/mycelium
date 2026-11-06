use super::{JwksProfileSignatureVerifier, ProfileJwksFetcher};
use crate::functions::fetch_profile_jwks;

use mycelium_base::utils::errors::MappedErrors;
use std::{collections::BTreeMap, env::vars_os, sync::Arc};

/// Whether a Rust downstream service verifies the gateway profile signature
///
/// Recommended: build it at startup with `from_environment()` and register it
/// as `web::Data`, so invalid settings stop the service. The gateway JWKS is
/// downloaded on the first signed request (requests fail closed until it is
/// available), so the service never depends on the gateway to start. Call
/// `prefetch().await` to download it at startup instead. Without
/// registration, `GatewayProfileData` reads the same settings lazily once and
/// fails requests closed when they are invalid (500).
///
#[derive(Clone, Debug)]
pub enum ProfileSignatureVerification {
    Disabled,
    Enabled(Arc<JwksProfileSignatureVerifier>),
}

impl ProfileSignatureVerification {
    /// Read and validate the environment; keys are fetched on first use
    pub fn from_environment() -> Result<Self, MappedErrors> {
        Self::from_variables(&environment_variables(), http_fetcher())
    }

    /// Opt-in: download the JWKS now (fails when the gateway is unreachable)
    #[tracing::instrument(name = "profile_signature_prefetch", skip_all)]
    pub async fn prefetch(&self) -> Result<(), MappedErrors> {
        if let Self::Enabled(verifier) = self {
            verifier.prefetch().await?;
        }

        Ok(())
    }

    /// Read environment-like variables with a given JWKS fetcher
    pub fn from_variables(
        variables: &BTreeMap<String, String>,
        fetcher: ProfileJwksFetcher,
    ) -> Result<Self, MappedErrors> {
        Ok(
            match JwksProfileSignatureVerifier::from_variables(
                variables, fetcher,
            )? {
                Some(verifier) => Self::Enabled(Arc::new(verifier)),
                None => Self::Disabled,
            },
        )
    }

    pub fn is_enabled(&self) -> bool {
        matches!(self, Self::Enabled(_))
    }
}

fn http_fetcher() -> ProfileJwksFetcher {
    Arc::new(fetch_profile_jwks)
}

/// The process environment; a non-UTF-8 value reads as empty, so it fails
/// validation instead of silently disappearing
fn environment_variables() -> BTreeMap<String, String> {
    vars_os()
        .filter_map(|(name, value)| {
            let name = name.into_string().ok()?;
            Some((name, value.into_string().unwrap_or_default()))
        })
        .collect()
}
