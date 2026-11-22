use super::PreparedFederation;

use async_trait::async_trait;
use chrono::Utc;
use myc_core::domain::{
    dtos::federation::{
        FederationPeer, LinkAcknowledgement, LinkConfirmation, LinkRevocation,
    },
    entities::{PeerClient, PeerLinkNotifier},
};
use myc_http_tools::{
    federation::{sign_peer_request, PeerRequest},
    settings::{FEDERATION_LINK_CONFIRM_PATH, FEDERATION_LINK_REVOKE_PATH},
};
use mycelium_base::utils::errors::{execution_err, MappedErrors};
use serde::Serialize;
use std::sync::Arc;
use uuid::Uuid;

/// Lifetime of a signed link call; the callee adds its clock tolerance
const LINK_CALL_LIFETIME_SECS: u64 = 60;

/// `PeerLinkNotifier` over signed peer requests (D-9)
pub(crate) struct SignedPeerLinkNotifier {
    prepared: PreparedFederation,
    peer_client: Arc<dyn PeerClient>,
}

impl SignedPeerLinkNotifier {
    pub(crate) fn new(
        prepared: PreparedFederation,
        peer_client: Arc<dyn PeerClient>,
    ) -> Self {
        Self {
            prepared,
            peer_client,
        }
    }

    async fn post<T: Serialize>(
        &self,
        peer: &FederationPeer,
        path: &str,
        payload: &T,
    ) -> Result<Vec<u8>, MappedErrors> {
        let body = serde_json::to_vec(payload)
            .map_err(|e| execution_err(format!("link call body: {e}")))?;
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
                lifetime_secs: LINK_CALL_LIFETIME_SECS,
            },
        )?;

        self.peer_client
            .post_signed(&peer.base_url, path, body, &signature)
            .await
    }
}

#[async_trait]
impl PeerLinkNotifier for SignedPeerLinkNotifier {
    #[tracing::instrument(name = "peer_confirm_link", skip_all)]
    async fn confirm_link(
        &self,
        peer: &FederationPeer,
        confirmation: LinkConfirmation,
    ) -> Result<LinkAcknowledgement, MappedErrors> {
        let answer = self
            .post(peer, FEDERATION_LINK_CONFIRM_PATH, &confirmation)
            .await?;

        serde_json::from_slice(&answer).map_err(|e| {
            execution_err(format!("peer link acknowledgement invalid: {e}"))
        })
    }

    #[tracing::instrument(name = "peer_revoke_link", skip_all)]
    async fn revoke_link(
        &self,
        peer: &FederationPeer,
        revocation: LinkRevocation,
    ) -> Result<(), MappedErrors> {
        self.post(peer, FEDERATION_LINK_REVOKE_PATH, &revocation)
            .await
            .map(|_| ())
    }
}
