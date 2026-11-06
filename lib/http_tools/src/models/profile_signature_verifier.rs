use super::{ProfileSignatureTolerance, TrustedProfileKeys};

use myc_core::domain::dtos::profile_signature::ProfileSignatureAudience;
use std::sync::Arc;

/// One verification pass: the trusted keys at an instant and the rules
///
/// Immutable and cheap to clone. The JWKS-backed verifier builds one per
/// request from its cached keys; the contract tests build one directly. The
/// audience is optional here so the shared vectors can exercise both cases;
/// a configured downstream always has one (see `JwksProfileSignatureVerifier`).
///
#[derive(Clone, Debug)]
pub struct ProfileSignatureVerifier {
    keys: Arc<TrustedProfileKeys>,
    tolerance: ProfileSignatureTolerance,
    expected_audience: Option<ProfileSignatureAudience>,
}

impl ProfileSignatureVerifier {
    pub fn new(
        keys: Arc<TrustedProfileKeys>,
        tolerance: ProfileSignatureTolerance,
        expected_audience: Option<ProfileSignatureAudience>,
    ) -> Self {
        Self {
            keys,
            tolerance,
            expected_audience,
        }
    }

    pub fn keys(&self) -> &TrustedProfileKeys {
        &self.keys
    }

    pub fn tolerance(&self) -> ProfileSignatureTolerance {
        self.tolerance
    }

    pub fn expected_audience(&self) -> Option<&ProfileSignatureAudience> {
        self.expected_audience.as_ref()
    }
}
