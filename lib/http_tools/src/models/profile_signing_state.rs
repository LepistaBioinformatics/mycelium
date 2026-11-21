use super::VersionedEd25519Signer;

use std::sync::Arc;

/// Whether the gateway signs downstream profiles, prepared once at boot
///
/// Registered as `web::Data` in every worker, including when disabled, so a
/// route that injects a profile can tell "signing is off" apart from "the
/// state was never registered" (an internal error, never a silent downgrade).
///
#[derive(Clone, Debug)]
pub enum ProfileSigningState {
    Disabled,
    Enabled(Arc<VersionedEd25519Signer>),
}

impl ProfileSigningState {
    pub fn is_enabled(&self) -> bool {
        matches!(self, Self::Enabled(_))
    }
}
