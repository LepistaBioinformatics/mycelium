use super::http_secret::HttpSecret;

use serde::{Deserialize, Serialize};
use utoipa::{ToResponse, ToSchema};
use uuid::Uuid;

/// This is the identifier where the service should reference the secret
///
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, ToResponse)]
#[serde(rename_all = "camelCase")]
pub enum SecretReference {
    /// Use the secret ID to reference a service secret
    Id {
        /// The secret id
        id: Uuid,

        /// If the secret exists
        exists: bool,

        /// The last time the secret was updated
        #[serde(skip_serializing_if = "Option::is_none")]
        last_updated: Option<String>,
    },

    /// Use the secret Name to reference a service secret
    Name {
        /// The secret name
        name: String,

        /// If the secret exists
        exists: bool,

        /// The last time the secret was updated
        #[serde(skip_serializing_if = "Option::is_none")]
        last_updated: Option<String>,
    },
}

/// The Service Secret
///
/// The service secret is a secret that is used by the service to authenticate
/// to the upstream service.
///
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, ToResponse)]
#[serde(rename_all = "camelCase")]
pub struct ServiceSecret {
    /// The service secret id
    pub id: Option<Uuid>,

    /// The service reference
    ///
    /// The service reference is the service that the secret is associated with.
    ///
    pub service_ref: SecretReference,

    /// The route secret unique name
    ///
    /// Routes should reference the route secrets by name, thus, its important
    /// to have a unique name in datastore.
    ///
    pub name: String,

    /// The route secret value
    ///
    /// The secret value should be a HttpSecret type and should be encrypted.
    /// Its important to note that the secret should be encrypted in the
    /// database and redacted on the response.
    ///
    secret: HttpSecret,
}
