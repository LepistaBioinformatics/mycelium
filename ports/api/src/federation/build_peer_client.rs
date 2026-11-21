use super::PreparedFederation;

use myc_core::domain::entities::PeerClient;
use myc_federation_client::HttpPeerClient;
use mycelium_base::utils::errors::MappedErrors;
use std::sync::Arc;

/// The client this instance calls its peers with, bounded by `peerTimeoutMs`
pub(crate) fn build_peer_client(
    prepared: &PreparedFederation,
) -> Result<Arc<dyn PeerClient>, MappedErrors> {
    let client = HttpPeerClient::new(prepared.config.peer_timeout_ms)?;

    Ok(Arc::new(client))
}
