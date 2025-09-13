/// An implementation from the alcholic_jwt crate.
///
/// This is a copy of the implementation from the alcholic_jwt crate.
///
/// The original implementation can be found at:
/// https://docs.rs/alcoholic_jwt/latest/alcoholic_jwt/
///
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};

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
