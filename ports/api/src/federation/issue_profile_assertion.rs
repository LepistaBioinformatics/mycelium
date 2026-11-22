use super::PreparedFederation;

use chrono::Utc;
use myc_core::domain::dtos::federation::{
    FederationPeer, ProfileAssertion, SubjectRef, PROFILE_ASSERTION_KIND,
};
use myc_http_tools::federation::{sign_assertion, AssertionClaims};
use mycelium_base::utils::errors::MappedErrors;
use uuid::Uuid;

/// Sign this issuer's answer about `subject_ref` for the asking peer, valid
/// for `assertionTtlSecs` (D-1, FED-14)
pub(crate) fn issue_profile_assertion(
    prepared: &PreparedFederation,
    peer: &FederationPeer,
    subject_ref: &SubjectRef,
    payload: ProfileAssertion,
) -> Result<String, MappedErrors> {
    let now = Utc::now().timestamp().max(0) as u64;

    sign_assertion(
        &prepared.signing.primary,
        &AssertionClaims {
            iss: prepared.instance_id,
            aud: peer.instance_id,
            sub: subject_ref.as_str().to_string(),
            iat: now,
            exp: now + prepared.config.assertion_ttl_secs,
            jti: Uuid::new_v4(),
            kind: PROFILE_ASSERTION_KIND.to_string(),
            payload,
        },
    )
}
