use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A peer a user may link an account with, as shown to that user (FED-43)
///
/// Only the identity and the name: never the peer's status, keys or policy.
///
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkablePeer {
    pub instance_id: Uuid,
    pub display_name: String,
}
