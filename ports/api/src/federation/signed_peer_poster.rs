use super::PreparedFederation;

use chrono::Utc;
use myc_core::domain::{
    dtos::federation::FederationPeer, entities::PeerClient,
};
use myc_http_tools::federation::{sign_peer_request, PeerRequest};
use mycelium_base::utils::errors::{execution_err, MappedErrors};
use serde::Serialize;
use std::sync::Arc;
use uuid::Uuid;

/// Lifetime of a signed peer call; the callee adds its clock tolerance
const PEER_CALL_LIFETIME_SECS: u64 = 60;

/// Posts JSON to a peer as a signed peer request (D-9)
#[derive(Clone)]
pub(crate) struct SignedPeerPoster {
    pub(crate) prepared: PreparedFederation,
    pub(crate) peer_client: Arc<dyn PeerClient>,
}

impl SignedPeerPoster {
    /// The response body of `POST <peer><path>`; every failure is an error
    pub(crate) async fn post<T: Serialize>(
        &self,
        peer: &FederationPeer,
        path: &str,
        payload: &T,
    ) -> Result<Vec<u8>, MappedErrors> {
        let body = serde_json::to_vec(payload)
            .map_err(|e| execution_err(format!("peer call body: {e}")))?;
        let jti = Uuid::new_v4().to_string();
        let signature = sign_peer_request(
            &self.prepared.signing.primary,
            &PeerRequest {
                iss: self.prepared.instance_id,
                aud: peer.instance_id,
                method: "POST",
                path,
                body: &body,
                jti: &jti,
                issued_at: Utc::now().timestamp().max(0) as u64,
                lifetime_secs: PEER_CALL_LIFETIME_SECS,
            },
        )?;

        self.peer_client
            .post_signed(&peer.base_url, path, body, &signature)
            .await
    }
}
