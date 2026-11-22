use super::{PeerRequest, PeerRequestClaims, PEER_REQUEST_TYP};
use crate::models::VersionedEd25519Signer;

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use mycelium_base::utils::errors::MappedErrors;
use sha2::{Digest, Sha256};

/// Sign a peer request as a compact EdDSA JWS with the caller's federation
/// key
///
/// Pure: the caller supplies the clock and the single-use `jti`.
///
pub fn sign_peer_request(
    signer: &VersionedEd25519Signer,
    request: &PeerRequest,
) -> Result<String, MappedErrors> {
    let claims = PeerRequestClaims {
        iss: request.iss,
        aud: request.aud,
        iat: request.issued_at,
        exp: request.issued_at + request.lifetime_secs,
        jti: request.jti.to_string(),
        htm: request.method.to_ascii_uppercase(),
        htp: request.path.to_string(),
        bh: body_hash(request.body),
    };

    signer.sign_typed_claims(Some(PEER_REQUEST_TYP), &claims)
}

/// SHA-256 of the body, Base64URL without padding
pub fn body_hash(body: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(body))
}
