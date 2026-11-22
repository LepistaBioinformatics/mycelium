use super::VerifiedAssertion;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A verified assertion kept in the assertion store (D-6, FED-15)
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredAssertion {
    pub link_id: Uuid,
    pub peer_id: Uuid,
    pub assertion: VerifiedAssertion,
}
