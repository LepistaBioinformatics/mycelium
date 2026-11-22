use super::{
    read_jws_segments::{decode_segment, header_kid, split_jws, JwsSegments},
    PeerRequestRejection,
};

use jsonwebtoken::{crypto::verify, Algorithm, DecodingKey};
use myc_core::domain::dtos::federation::PeerKey;
use serde_json::Value;

/// Check a compact JWS signed by a peer: EdDSA only, the expected `typ`, a
/// `kid` among the peer's stored keys and a valid signature
///
/// Returns the `kid` and the still-encoded payload; the claims are the
/// caller's to decode and check.
///
pub(super) fn verify_typed_jws<'a>(
    jws: &'a str,
    keys: &[PeerKey],
    typ: &str,
) -> Result<(String, &'a str), PeerRequestRejection> {
    let segments = split_jws(jws)?;
    let header = decode_segment(segments.header)?;
    check_header(&header, typ)?;

    let kid = header_kid(&header)?;
    verify_signature(&segments, &decoding_key_of(keys, &kid)?)?;

    Ok((kid, segments.payload))
}

fn verify_signature(
    segments: &JwsSegments,
    decoding_key: &DecodingKey,
) -> Result<(), PeerRequestRejection> {
    let input = segments.signing_input();

    match verify(
        segments.signature,
        input.as_bytes(),
        decoding_key,
        Algorithm::EdDSA,
    ) {
        Ok(true) => Ok(()),
        _ => Err(PeerRequestRejection::BadSignature),
    }
}

fn decoding_key_of(
    keys: &[PeerKey],
    kid: &str,
) -> Result<DecodingKey, PeerRequestRejection> {
    let key = keys
        .iter()
        .find(|key| key.kid == kid)
        .ok_or_else(|| PeerRequestRejection::UnknownKid(kid.to_string()))?;

    DecodingKey::from_ed_components(&key.public_key)
        .map_err(|_| PeerRequestRejection::BadSignature)
}

fn check_header(header: &Value, typ: &str) -> Result<(), PeerRequestRejection> {
    if header.get("alg").and_then(Value::as_str) != Some("EdDSA") {
        return Err(PeerRequestRejection::UnsupportedAlgorithm);
    }
    if header.get("typ").and_then(Value::as_str) != Some(typ) {
        return Err(PeerRequestRejection::WrongType);
    }

    Ok(())
}
