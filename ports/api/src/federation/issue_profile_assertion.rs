use super::PreparedFederation;

use chrono::Utc;
use myc_core::domain::dtos::federation::{
    FederationPeer, ProfileAssertion, SubjectRef, PROFILE_ASSERTION_KIND,
};
use myc_http_tools::federation::{sign_assertion, AssertionClaims};
use mycelium_base::utils::errors::MappedErrors;
use serde::Serialize;
use uuid::Uuid;

/// Sign an assertion of any kind for the asking peer, valid for
/// `assertionTtlSecs` (D-1, FED-38): only `subject`, `kind` and the payload
/// differ between kinds
pub(crate) fn issue_assertion<P: Serialize>(
    prepared: &PreparedFederation,
    peer: &FederationPeer,
    subject: String,
    kind: &str,
    payload: P,
) -> Result<String, MappedErrors> {
    let now = Utc::now().timestamp().max(0) as u64;

    sign_assertion(
        &prepared.signing.primary,
        &AssertionClaims {
            iss: prepared.instance_id,
            aud: peer.instance_id,
            sub: subject,
            iat: now,
            exp: now + prepared.config.assertion_ttl_secs,
            jti: Uuid::new_v4(),
            kind: kind.to_string(),
            payload,
        },
    )
}

/// Sign this issuer's answer about `subject_ref` for the asking peer
/// (FED-14)
pub(crate) fn issue_profile_assertion(
    prepared: &PreparedFederation,
    peer: &FederationPeer,
    subject_ref: &SubjectRef,
    payload: ProfileAssertion,
) -> Result<String, MappedErrors> {
    issue_assertion(
        prepared,
        peer,
        subject_ref.as_str().to_string(),
        PROFILE_ASSERTION_KIND,
        payload,
    )
}
