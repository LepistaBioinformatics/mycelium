use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// One federation public key as a JSON Web Key (RFC 7517 / RFC 8037)
///
/// Carries only public members. `core` cannot depend on `http_tools`, so the
/// gateway maps its `Ed25519PublicJwk` into this shape.
///
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FederationPublicJwk {
    pub kty: String,
    pub crv: String,
    pub x: String,
    pub kid: String,
    #[serde(rename = "use")]
    pub key_use: String,
    pub alg: String,
}

/// The public self-description of an instance (FED-03)
///
/// Served without authentication at `/.well-known/mycelium-instance`. Never
/// carries private material.
///
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceSelfDescription {
    pub instance_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    pub base_url: String,
    pub version: String,
    pub assertion_kinds: Vec<String>,
    pub keys: Vec<FederationPublicJwk>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role_catalog_url: Option<String>,
}
