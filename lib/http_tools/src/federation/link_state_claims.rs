use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// `typ` of the protected header of a link redirect state
pub const LINK_STATE_TYP: &str = "mycelium-link-state+jwt";

/// The redirect state a consumer signs when it sends a user to the issuer to
/// link accounts (FED-46)
///
/// Bound to the consumer's single-use link request (`jti`) and to the email
/// of the user who started it, so the issuer can apply the same-email rule
/// (FED-45) and send the browser back only to `return_url`.
///
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkStateClaims {
    /// Consumer `instance_id`
    pub iss: Uuid,
    /// Issuer `instance_id`
    pub aud: Uuid,
    pub iat: u64,
    pub exp: u64,
    /// The consumer's link request id
    pub jti: Uuid,
    /// Email of the consumer account being linked
    pub email: String,
    /// The consumer page the browser returns to
    pub return_url: String,
}

/// A verified link state: its claims and the key that signed it
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedLinkState {
    pub kid: String,
    pub claims: LinkStateClaims,
}

pub struct ExpectedLinkState {
    pub audience: Uuid,
    pub now: u64,
    pub tolerance_secs: u64,
}
