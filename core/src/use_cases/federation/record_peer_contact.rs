use super::{next_peer_status, ContactOutcome};
use crate::domain::{
    dtos::federation::FederationPeer, entities::FederationPeerWrite,
};

use mycelium_base::utils::errors::MappedErrors;

/// Count a call to a peer in its circuit breaker (FED-21)
///
/// A quarantined or untrusted peer keeps its status: `settle_status` never
/// overrides an operator decision.
///
#[tracing::instrument(name = "record_peer_contact", skip_all)]
pub async fn record_peer_contact(
    peer: &FederationPeer,
    outcome: ContactOutcome,
    max_failures_before_open: u32,
    peer_write_repo: Box<&dyn FederationPeerWrite>,
) -> Result<(), MappedErrors> {
    let failures = match outcome {
        ContactOutcome::Success => {
            peer_write_repo.record_success(peer.id).await?;
            0
        }
        ContactOutcome::Failure => {
            peer_write_repo.record_failure(peer.id).await?
        }
    };

    let status = next_peer_status(
        peer.status,
        outcome,
        failures,
        max_failures_before_open,
    );
    peer_write_repo.settle_status(peer.id, status).await?;

    Ok(())
}
