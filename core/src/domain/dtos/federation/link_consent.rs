use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// What a consumer asked the issuer to link, kept on the issuer until the
/// user confirms (FED-46)
///
/// `local_user_id` is set once the user has logged in on the issuer.
///
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkConsent {
    pub id: Uuid,
    pub peer_id: Uuid,

    /// The consumer's link request, the `jti` of the redirect state
    pub peer_request_id: Uuid,

    /// The email of the consumer's account, signed in the redirect state
    pub peer_email: String,

    /// Where the browser goes back to; always under the peer's `base_url`
    pub return_url: String,

    pub local_user_id: Option<Uuid>,
    pub expires_at: DateTime<Local>,
}

/// A consent to record when a redirect state is accepted
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewLinkConsent {
    pub peer_id: Uuid,
    pub peer_request_id: Uuid,
    pub peer_email: String,
    pub return_url: String,
    pub expires_at: DateTime<Local>,
}
