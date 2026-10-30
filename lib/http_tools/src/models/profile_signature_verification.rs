use super::ProfileSignatureVerifier;

use mycelium_base::utils::errors::MappedErrors;
use std::sync::Arc;

/// Whether a Rust downstream service verifies the gateway profile signature
///
/// Register it as `web::Data` at startup (built with `from_environment`) so a
/// bad configuration stops the service before it serves traffic. Without
/// registration, `GatewayProfileData` loads the same settings lazily once and
/// fails requests closed (500) when they are invalid.
///
#[derive(Clone, Debug)]
pub enum ProfileSignatureVerification {
    Disabled,
    Enabled(Arc<ProfileSignatureVerifier>),
}

impl ProfileSignatureVerification {
    pub fn from_environment() -> Result<Self, MappedErrors> {
        Ok(match ProfileSignatureVerifier::from_environment()? {
            Some(verifier) => Self::Enabled(Arc::new(verifier)),
            None => Self::Disabled,
        })
    }

    pub fn is_enabled(&self) -> bool {
        matches!(self, Self::Enabled(_))
    }
}
