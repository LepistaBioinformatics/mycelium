use super::{PeerRequestClaims, PeerRequestRejection};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use serde_json::Value;
use uuid::Uuid;

/// The three segments of a compact JWS
pub(super) struct JwsSegments<'a> {
    pub(super) header: &'a str,
    pub(super) payload: &'a str,
    pub(super) signature: &'a str,
}

impl<'a> JwsSegments<'a> {
    /// What the signature covers: `header.payload`
    pub(super) fn signing_input(&self) -> String {
        format!("{}.{}", self.header, self.payload)
    }
}

pub(super) fn split_jws(
    jws: &str,
) -> Result<JwsSegments<'_>, PeerRequestRejection> {
    let mut parts = jws.split('.');

    match (parts.next(), parts.next(), parts.next(), parts.next()) {
        (Some(header), Some(payload), Some(signature), None)
            if !header.is_empty()
                && !payload.is_empty()
                && !signature.is_empty() =>
        {
            Ok(JwsSegments {
                header,
                payload,
                signature,
            })
        }
        _ => Err(PeerRequestRejection::Malformed),
    }
}

pub(super) fn decode_segment(
    segment: &str,
) -> Result<Value, PeerRequestRejection> {
    let bytes = URL_SAFE_NO_PAD
        .decode(segment)
        .map_err(|_| PeerRequestRejection::Malformed)?;

    serde_json::from_slice(&bytes).map_err(|_| PeerRequestRejection::Malformed)
}

pub(super) fn header_kid(
    header: &Value,
) -> Result<String, PeerRequestRejection> {
    header
        .get("kid")
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or(PeerRequestRejection::Malformed)
}

pub(super) fn decode_claims(
    segment: &str,
) -> Result<PeerRequestClaims, PeerRequestRejection> {
    serde_json::from_value(decode_segment(segment)?)
        .map_err(|_| PeerRequestRejection::Malformed)
}

/// The caller and key a request *claims*, read without verification
///
/// Only for finding the caller's stored keys; nothing here is trusted until
/// `verify_peer_request` succeeds.
///
pub fn peek_peer_request_issuer(
    jws: &str,
) -> Result<(Uuid, String), PeerRequestRejection> {
    let segments = split_jws(jws)?;
    let kid = header_kid(&decode_segment(segments.header)?)?;

    Ok((decode_claims(segments.payload)?.iss, kid))
}
