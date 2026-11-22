use myc_core::domain::dtos::federation::ProfileAssertion;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// `typ` of the protected header of a federated assertion
pub const ASSERTION_TYP: &str = "mycelium-assertion+jwt";

/// A peer's signed answer about one linked subject (design §4, FED-14)
///
/// Bound to the asking instance (`aud`) and to the link's opaque reference
/// (`sub`), so it cannot be replayed to a third peer or for another subject.
///
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AssertionClaims<P = ProfileAssertion> {
    /// Issuer `instance_id`
    pub iss: Uuid,
    /// Requester `instance_id`
    pub aud: Uuid,
    /// The issuer's `local_subject_ref` of the link
    pub sub: String,
    pub iat: u64,
    pub exp: u64,
    pub jti: Uuid,
    pub kind: String,
    pub payload: P,
}

/// The kind alone, read before the payload so another kind is recognized as
/// such rather than as a malformed payload (FED-41)
#[derive(Deserialize)]
pub(super) struct AssertionKindOnly {
    pub kind: String,
}

/// What a consumer expects of an assertion it asked for
pub struct ExpectedAssertion {
    pub issuer: Uuid,
    pub audience: Uuid,
    pub subject: String,
    pub kind: String,
    pub now: u64,
    pub tolerance_secs: u64,
}
