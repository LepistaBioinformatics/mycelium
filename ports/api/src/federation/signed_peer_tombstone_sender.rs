use super::SignedPeerPoster;

use async_trait::async_trait;
use myc_core::domain::{
    dtos::federation::{FederationPeer, Tombstone},
    entities::PeerTombstoneSender,
};
use myc_http_tools::settings::FEDERATION_TOMBSTONE_PATH;
use mycelium_base::utils::errors::MappedErrors;

/// `PeerTombstoneSender` over signed peer requests (D-9)
pub(crate) struct SignedPeerTombstoneSender {
    pub(crate) poster: SignedPeerPoster,
}

#[async_trait]
impl PeerTombstoneSender for SignedPeerTombstoneSender {
    #[tracing::instrument(name = "peer_push_tombstone", skip_all)]
    async fn push(
        &self,
        peer: &FederationPeer,
        tombstone: Tombstone,
    ) -> Result<(), MappedErrors> {
        self.poster
            .post(peer, FEDERATION_TOMBSTONE_PATH, &tombstone)
            .await
            .map(|_| ())
    }
}
