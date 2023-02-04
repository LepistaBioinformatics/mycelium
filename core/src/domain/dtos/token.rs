use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Represents the requests validation token.
///
/// Validation tokens are used to check the validity of the Profile retrieved
/// the every request submitted to the cluster.
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Token {
    /// This token should be used as the request identifier.
    pub token: Uuid,

    /// This field should contains the name of the requester service. Such name
    /// should be used to check if the token was consumed by the downstream
    /// route, enabled by the service router.
    pub own_service: String,
}

impl Token {
    pub async fn new_undated_token(own_service: String) -> Self {
        Self {
            token: Uuid::new_v4(),
            own_service,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Claims {
    pub email: String,
    pub exp: usize,
}
