use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A single-use link initiation on the consumer (FED-42)
///
/// Its id is the `jti` of the signed redirect state. `redirected_at` is set
/// when the initiation page sends the browser to the issuer (the URL is spent
/// from then on); `consumed_at` when the issuer confirms the link.
///
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkRequest {
    pub id: Uuid,
    pub local_user_id: Uuid,
    pub peer_id: Uuid,
    pub expires_at: DateTime<Local>,
    pub redirected_at: Option<DateTime<Local>>,
}

/// A link initiation to record; only the hash of the URL secret is kept
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewLinkRequest {
    pub local_user_id: Uuid,
    pub peer_id: Uuid,
    pub secret_hash: String,
    pub expires_at: DateTime<Local>,
}
