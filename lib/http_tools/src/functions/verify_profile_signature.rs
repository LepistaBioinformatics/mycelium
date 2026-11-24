use super::{
    check_profile_signature_claims, decode_and_decompress_profile_from_base64,
    read_profile_signature_claims,
};
use crate::{
    models::ProfileSignatureVerifier, settings::MAX_PROFILE_SIGNATURE_BYTES,
};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use jsonwebtoken::{crypto::verify, Algorithm, DecodingKey};
use myc_core::domain::dtos::{
    profile::Profile,
    profile_signature::{ProfileSignatureRejection, UnixTimestampSeconds},
};
use serde_json::{Map, Value};

/// JOSE members a strict verifier must never honor
const FORBIDDEN_HEADER_MEMBERS: [&str; 6] =
    ["crit", "b64", "jku", "jwk", "x5u", "x5c"];

/// The three still-encoded segments of a compact JWS
struct CompactJws<'a> {
    header: &'a str,
    payload: &'a str,
    signature: &'a str,
}

/// Verify that `signature_header` authenticates `profile_header`
///
/// Strict, local check of the contract shared with every SDK: compact JWS,
/// `alg: EdDSA`, trusted canonical `kid`, claims `{iat, ph, aud}`, symmetric
/// inclusive time window, hash of the exact header value and, when
/// configured, the expected audience. Checks run in the shared precedence, so
/// the returned reason matches the Python, JavaScript and Go verifiers. The
/// profile is not decoded here; see `verify_and_decode_profile`.
///
#[tracing::instrument(name = "verify_profile_signature", skip_all)]
pub fn verify_profile_signature(
    profile_header: &str,
    signature_header: &str,
    verifier: &ProfileSignatureVerifier,
    now: UnixTimestampSeconds,
) -> Result<(), ProfileSignatureRejection> {
    if profile_header.is_empty() {
        return Err(ProfileSignatureRejection::MissingProfile);
    }

    let token = split_token(signature_header)?;
    let header = decode_object(token.header)
        .ok_or(ProfileSignatureRejection::MalformedSignature)?;
    let key = select_key(&header, verifier)?;
    let payload = verify_segments(&token, key)?;
    let claims = read_profile_signature_claims(&payload)?;

    check_profile_signature_claims(&claims, profile_header, verifier, now)
}

/// Verify the signature, then decode the profile (`invalid_profile`)
#[tracing::instrument(name = "verify_and_decode_profile", skip_all)]
pub fn verify_and_decode_profile(
    profile_header: &str,
    signature_header: &str,
    verifier: &ProfileSignatureVerifier,
    now: UnixTimestampSeconds,
) -> Result<Profile, ProfileSignatureRejection> {
    verify_profile_signature(profile_header, signature_header, verifier, now)?;

    decode_and_decompress_profile_from_base64(profile_header.to_string())
        .map_err(|_| ProfileSignatureRejection::InvalidProfile)
}

fn split_token(
    signature_header: &str,
) -> Result<CompactJws<'_>, ProfileSignatureRejection> {
    if signature_header.is_empty() {
        return Err(ProfileSignatureRejection::MissingSignature);
    }

    let parts: Vec<&str> = signature_header.split('.').collect();
    let [header, payload, signature] = parts.as_slice() else {
        return Err(ProfileSignatureRejection::MalformedSignature);
    };

    if signature_header.len() > MAX_PROFILE_SIGNATURE_BYTES {
        return Err(ProfileSignatureRejection::MalformedSignature);
    }

    Ok(CompactJws {
        header,
        payload,
        signature,
    })
}

fn decode_object(segment: &str) -> Option<Map<String, Value>> {
    let bytes = URL_SAFE_NO_PAD.decode(segment).ok()?;

    match serde_json::from_slice::<Value>(&bytes).ok()? {
        Value::Object(members) => Some(members),
        _ => None,
    }
}

fn select_key<'a>(
    header: &Map<String, Value>,
    verifier: &'a ProfileSignatureVerifier,
) -> Result<&'a DecodingKey, ProfileSignatureRejection> {
    if header.get("alg").and_then(Value::as_str) != Some("EdDSA") {
        return Err(ProfileSignatureRejection::UnsupportedAlgorithm);
    }

    if FORBIDDEN_HEADER_MEMBERS
        .iter()
        .any(|member| header.contains_key(*member))
    {
        return Err(ProfileSignatureRejection::MalformedSignature);
    }

    header
        .get("kid")
        .and_then(Value::as_str)
        .and_then(|kid| verifier.keys().key_for(kid))
        .ok_or(ProfileSignatureRejection::UnknownKey)
}

fn verify_segments(
    token: &CompactJws<'_>,
    key: &DecodingKey,
) -> Result<Vec<u8>, ProfileSignatureRejection> {
    let payload = URL_SAFE_NO_PAD
        .decode(token.payload)
        .map_err(|_| ProfileSignatureRejection::MalformedSignature)?;
    URL_SAFE_NO_PAD
        .decode(token.signature)
        .map_err(|_| ProfileSignatureRejection::MalformedSignature)?;

    let message = format!("{}.{}", token.header, token.payload);
    let is_authentic =
        verify(token.signature, message.as_bytes(), key, Algorithm::EdDSA)
            .unwrap_or(false);

    is_authentic
        .then_some(payload)
        .ok_or(ProfileSignatureRejection::InvalidSignature)
}
