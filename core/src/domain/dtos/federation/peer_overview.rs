use super::FederationPeer;

use serde::{Deserialize, Serialize};

/// What an operator sees of a peer: the peer itself and the `kid`s of the
/// keys stored for it (FED-33)
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PeerOverview {
    #[serde(flatten)]
    pub peer: FederationPeer,
    pub key_ids: Vec<String>,
}
