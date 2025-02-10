/// An implementation from the alcholic_jwt crate.
///
/// This is a copy of the implementation from the alcholic_jwt crate.
///
/// The original implementation can be found at:
/// https://docs.rs/alcoholic_jwt/latest/alcoholic_jwt/
///
use base64::DecodeError;
use openssl::error::ErrorStack;
use serde::{Deserialize, Serialize};
use std::{
    error::Error,
    fmt::{self, Display},
};

#[derive(Clone, Serialize, Deserialize, Debug)]
enum KeyAlgorithm {
    RS256,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
enum KeyType {
    RSA,
}

/// Representation of a single JSON Web Key. See [RFC
/// 7517](https://tools.ietf.org/html/rfc7517#section-4).
#[allow(dead_code)] // kty & alg only constrain deserialisation, but aren't used
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct JWK {
    kty: KeyType,
    alg: Option<KeyAlgorithm>,
    kid: Option<String>,

    // Shared modulus
    pub(crate) n: String,

    // Public key exponent
    pub(crate) e: String,

    pub(crate) x5c: Option<Vec<String>>,
}

impl Display for JWK {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "JWK: {:?}", self)
    }
}

/// Representation of a set of JSON Web Keys. See [RFC
/// 7517](https://tools.ietf.org/html/rfc7517#section-5).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct JWKS {
    // This is a vector instead of some kind of map-like structure
    // because key IDs are in fact optional.
    //
    // Technically having multiple keys with the same KID would not
    // violate the JWKS-definition either, but behaviour in that case
    // is unspecified.
    keys: Vec<JWK>,
}

impl JWKS {
    /// Attempt to find a JWK by its key ID.
    pub fn find(&self, kid: &str) -> Option<&JWK> {
        self.keys.iter().find(|jwk| jwk.kid == Some(kid.into()))
    }
}

/// Possible results of a token validation.
#[derive(Debug)]
pub(crate) enum ValidationError {
    /// Invalid number of token components (not a JWT?)
    //InvalidComponents,

    /// Token segments had invalid base64-encoding.
    InvalidBase64(DecodeError),

    /// Decoding of the provided JWK failed.
    //InvalidJWK,

    /// Signature validation failed, i.e. because of a non-matching
    /// public key.
    //InvalidSignature,

    /// An OpenSSL operation failed along the way at a point at which
    /// a more specific error variant could not be constructed.
    OpenSSL(ErrorStack),

    /// JSON decoding into a provided type failed.
    JSON(serde_json::Error),
    // One or more claim validations failed. This variant contains
    // human-readable validation errors.
    //InvalidClaims(Vec<&'static str>),
}

impl Error for ValidationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ValidationError::InvalidBase64(e) => Some(e),
            ValidationError::OpenSSL(e) => Some(e),
            ValidationError::JSON(e) => Some(e),
            //ValidationError::InvalidComponents
            //| ValidationError::InvalidJWK
            //| ValidationError::InvalidSignature
            //| ValidationError::InvalidClaims(_) => None,
            //| ,
            //ValidationError::InvalidClaims(_) => None,
        }
    }
}

impl Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            //ValidationError::InvalidComponents => {
            //    f.write_str("Invalid number of token components in JWT")
            //}
            ValidationError::InvalidBase64(_) => {
                f.write_str("Invalid Base64 encoding in JWT")
            }
            //ValidationError::InvalidJWK => f.write_str("JWK decoding failed"),
            //ValidationError::InvalidSignature => {
            //    f.write_str("JWT signature validation failed")
            //}
            ValidationError::OpenSSL(e) => write!(f, "SSL error: {}", e),
            ValidationError::JSON(e) => write!(f, "JSON error: {}", e),
            //ValidationError::InvalidClaims(errs) => {
            //    write!(f, "Invalid claims: {}", errs.join(", "))
            //}
        }
    }
}

impl From<ErrorStack> for ValidationError {
    fn from(err: ErrorStack) -> Self {
        ValidationError::OpenSSL(err)
    }
}

impl From<serde_json::Error> for ValidationError {
    fn from(err: serde_json::Error) -> Self {
        ValidationError::JSON(err)
    }
}

impl From<DecodeError> for ValidationError {
    fn from(err: DecodeError) -> Self {
        ValidationError::InvalidBase64(err)
    }
}
