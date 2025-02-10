use jwt::claims::SecondsSinceEpoch;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub(crate) enum Audience {
    Single(String),
    Multiple(Vec<String>),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct GenericAccessTokenClaims {
    #[serde(rename = "iss", skip_serializing_if = "Option::is_none")]
    pub(crate) issuer: Option<String>,

    #[serde(rename = "sub")]
    pub(crate) subject: String,

    #[serde(rename = "aud")]
    pub(crate) audience: Audience,

    #[serde(rename = "exp", skip_serializing_if = "Option::is_none")]
    pub(crate) expiration: Option<SecondsSinceEpoch>,

    #[serde(rename = "nbf", skip_serializing_if = "Option::is_none")]
    pub(crate) not_before: Option<SecondsSinceEpoch>,

    #[serde(rename = "iat")]
    pub(crate) issued_at: SecondsSinceEpoch,

    #[serde(rename = "jti", skip_serializing_if = "Option::is_none")]
    pub(crate) json_web_token_id: Option<String>,

    #[serde(rename = "email", skip_serializing_if = "Option::is_none")]
    pub(crate) email: Option<String>,

    #[serde(flatten)]
    pub(crate) fields: HashMap<String, Value>,
}

/// Generic claims
///
/// This struct is used to represent the generic claims of the token. It is
/// needed to parse tokens from multiple identity providers.
///
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct GenericIDTokenClaims {
    // ? -----------------------------------------------------------------------
    // ? Microsoft claim fields
    // ? -----------------------------------------------------------------------
    /// Field `upn` is the Microsoft email address
    #[serde(rename = "upn")]
    pub(crate) upn: Option<String>,

    /// Field `unique_name` is the Microsoft name
    #[serde(rename = "unique_name")]
    pub(crate) unique_name: Option<String>,

    // ? -----------------------------------------------------------------------
    // ? Google claim fields
    // ? -----------------------------------------------------------------------
    /// Google email address
    #[serde(rename = "email")]
    pub(crate) email: Option<String>,

    #[serde(rename = "aud")]
    pub(crate) audience: Audience,

    // ? -----------------------------------------------------------------------
    // ? Other providers claim fields
    // ? -----------------------------------------------------------------------

    #[serde(flatten)]
    pub(crate) fields: HashMap<String, Value>,
}
