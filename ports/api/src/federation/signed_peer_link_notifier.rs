use super::{PreparedFederation, SignedPeerPoster};

use async_trait::async_trait;
use myc_core::domain::{
    dtos::federation::{
        FederationPeer, LinkAcknowledgement, LinkConfirmation, LinkRevocation,
    },
    entities::{PeerClient, PeerLinkNotifier},
};
use myc_http_tools::settings::{
    FEDERATION_LINK_CONFIRM_PATH, FEDERATION_LINK_REVOKE_PATH,
};
use mycelium_base::utils::errors::{execution_err, MappedErrors};
use std::sync::Arc;

/// `PeerLinkNotifier` over signed peer requests (D-9)
pub(crate) struct SignedPeerLinkNotifier {
    poster: SignedPeerPoster,
}

impl SignedPeerLinkNotifier {
    pub(crate) fn new(
        prepared: PreparedFederation,
        peer_client: Arc<dyn PeerClient>,
    ) -> Self {
        Self {
            poster: SignedPeerPoster {
                prepared,
                peer_client,
            },
        }
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
            .poster
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
        self.poster
            .post(peer, FEDERATION_LINK_REVOKE_PATH, &revocation)
            .await
            .map(|_| ())
    }
}
