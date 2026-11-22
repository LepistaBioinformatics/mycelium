use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// `typ` of the protected header of a signed peer request
pub const PEER_REQUEST_TYP: &str = "mycelium-peer-request+jwt";

/// The claims of a signed peer request (D-9, FED-09)
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeerRequestClaims {
    /// Caller `instance_id`
    pub iss: Uuid,
    /// Callee `instance_id`
    pub aud: Uuid,
    pub iat: u64,
    pub exp: u64,
    /// Single-use request id
    pub jti: String,
    /// HTTP method, upper case
    pub htm: String,
    /// HTTP path, without the query string
    pub htp: String,
    /// SHA-256 of the request body, Base64URL without padding
    pub bh: String,
}

/// A verified peer request: its claims and the key that signed it
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedPeerRequest {
    pub kid: String,
    pub claims: PeerRequestClaims,
}

/// What a caller signs
pub struct PeerRequest<'a> {
    pub iss: Uuid,
    pub aud: Uuid,
    pub method: &'a str,
    pub path: &'a str,
    pub body: &'a [u8],
    pub jti: &'a str,
    pub issued_at: u64,
    pub lifetime_secs: u64,
}

pub struct ExpectedPeerRequest<'a> {
    pub audience: Uuid,
    pub method: &'a str,
    pub path: &'a str,
    pub body: &'a [u8],
    pub now: u64,
    pub tolerance_secs: u64,
}
