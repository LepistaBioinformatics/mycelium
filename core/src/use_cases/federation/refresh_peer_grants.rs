use super::{
    audit_peer_resolution, guard_namespace_collisions, record_peer_contact,
    ContactOutcome, ResolutionLimits, ResolutionPorts,
};
use crate::domain::dtos::{
    federation::{
        FederatedGrants, FederationPeer, IdentityLink, PeerGrantAnswer,
        PeerOutcome, PeerResolution, StoredAssertion, SubjectRef,
        VerifiedAssertion,
    },
    written_by::WrittenBy,
};

use chrono::Local;
use mycelium_base::utils::errors::MappedErrors;

/// Ask one linked peer for the subject's grants, then verify, guard, store,
/// count the call in the peer's circuit and audit it (FED-13…16, FED-54)
///
/// A call that fails or does not finish before the deadline is a partial
/// outcome, never an error.
///
#[tracing::instrument(name = "refresh_peer_grants", skip_all)]
pub async fn refresh_peer_grants(
    link: &IdentityLink,
    subject_ref: SubjectRef,
    peer: &FederationPeer,
    author: &WrittenBy,
    limits: ResolutionLimits,
    ports: &ResolutionPorts<'_>,
) -> Result<PeerOutcome, MappedErrors> {
    let asked = tokio::time::timeout(
        limits.deadline,
        ports.peers.resolver.resolve(peer, subject_ref),
    )
    .await;

    let (resolution, grants) = match asked {
        Err(_) => (unreachable("deadline exceeded"), empty()),
        Ok(Err(err)) => (unreachable(&err.to_string()), empty()),
        Ok(Ok(answer)) => settle(link, peer, answer, limits, ports).await?,
    };

    let outcome = outcome_of(peer, resolution, grants);
    record_contact(peer, &outcome, limits, ports).await?;
    audit_peer_resolution(link.id, &outcome, author, Box::new(ports.audit))
        .await;

    Ok(outcome)
}

async fn settle(
    link: &IdentityLink,
    peer: &FederationPeer,
    answer: PeerGrantAnswer,
    limits: ResolutionLimits,
    ports: &ResolutionPorts<'_>,
) -> Result<(PeerResolution, FederatedGrants), MappedErrors> {
    let assertion = match answer {
        PeerGrantAnswer::Unknown => {
            return Ok((PeerResolution::Unknown, empty()))
        }
        PeerGrantAnswer::Invalid(reason) => {
            return Ok((PeerResolution::Rejected(reason), empty()))
        }
        PeerGrantAnswer::Verified(assertion) => assertion,
    };

    if let Some(reason) = guard(peer, &assertion, ports).await? {
        return Ok((PeerResolution::Rejected(reason), empty()));
    }

    let grants = assertion.payload.grants.clone();
    let latest = Local::now() + limits.assertion_ttl;
    store(link, assertion.kept_until(latest), ports).await?;
    Ok((PeerResolution::Answered, grants))
}

async fn guard(
    peer: &FederationPeer,
    assertion: &VerifiedAssertion,
    ports: &ResolutionPorts<'_>,
) -> Result<Option<String>, MappedErrors> {
    guard_namespace_collisions(
        peer.id,
        &assertion.payload,
        Box::new(ports.guard.probe),
        Box::new(ports.guard.claims),
    )
    .await
}

async fn store(
    link: &IdentityLink,
    assertion: VerifiedAssertion,
    ports: &ResolutionPorts<'_>,
) -> Result<(), MappedErrors> {
    ports
        .store
        .write
        .store(StoredAssertion {
            link_id: link.id,
            peer_id: link.peer_id,
            assertion,
        })
        .await
}

async fn record_contact(
    peer: &FederationPeer,
    outcome: &PeerOutcome,
    limits: ResolutionLimits,
    ports: &ResolutionPorts<'_>,
) -> Result<(), MappedErrors> {
    let contact = match outcome.resolution.is_partial() {
        true => ContactOutcome::Failure,
        false => ContactOutcome::Success,
    };

    record_peer_contact(
        peer,
        contact,
        limits.max_failures_before_open,
        Box::new(ports.peers.write),
    )
    .await
}

fn outcome_of(
    peer: &FederationPeer,
    resolution: PeerResolution,
    grants: FederatedGrants,
) -> PeerOutcome {
    PeerOutcome {
        peer_id: peer.id,
        instance_id: peer.instance_id,
        resolution,
        grants,
    }
}

fn unreachable(reason: &str) -> PeerResolution {
    PeerResolution::Unreachable(reason.to_string())
}

fn empty() -> FederatedGrants {
    FederatedGrants::default()
}
