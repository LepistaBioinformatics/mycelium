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
/// count the call in the peer's circuit and audit it (FED-13…18, FED-54)
///
/// `stored` holds the grants of the link's unexpired assertion, if any. A
/// call that fails or misses the deadline serves them as stale (FED-17), or
/// contributes nothing without them (FED-18); never an error. A peer that
/// answers "unknown", or whose answer fails a check, no longer vouches for
/// the stored grants: they are forgotten. An answer issued before the link's
/// tombstone is not stored and contributes nothing (FED-23).
///
#[tracing::instrument(name = "refresh_peer_grants", skip_all)]
pub async fn refresh_peer_grants(
    link: IdentityLink,
    peer: FederationPeer,
    stored: Option<FederatedGrants>,
    author: &WrittenBy,
    limits: ResolutionLimits,
    ports: &ResolutionPorts<'_>,
) -> Result<PeerOutcome, MappedErrors> {
    let Some(subject_ref) = link.peer_subject_ref.clone() else {
        let reason = "link has no reference".to_string();
        return Ok(outcome_of(&peer, PeerResolution::Skipped(reason), empty()));
    };

    let (resolution, grants) =
        match ask(&peer, subject_ref, limits, ports).await {
            Err(reason) => fall_back(stored, reason),
            Ok(answer) => settle(&link, &peer, answer, limits, ports).await?,
        };

    let outcome = outcome_of(&peer, resolution, grants);
    record_contact(&peer, &outcome, limits, ports).await?;
    audit_peer_resolution(link.id, &outcome, author, Box::new(ports.audit))
        .await;

    Ok(outcome)
}

/// The peer's answer, or why there is none
async fn ask(
    peer: &FederationPeer,
    subject_ref: SubjectRef,
    limits: ResolutionLimits,
    ports: &ResolutionPorts<'_>,
) -> Result<PeerGrantAnswer, String> {
    let asked = tokio::time::timeout(
        limits.deadline,
        ports.peers.resolver.resolve(peer, subject_ref),
    )
    .await;

    match asked {
        Err(_) => Err("deadline exceeded".to_string()),
        Ok(Err(err)) => Err(err.to_string()),
        Ok(Ok(answer)) => Ok(answer),
    }
}

fn fall_back(
    stored: Option<FederatedGrants>,
    reason: String,
) -> (PeerResolution, FederatedGrants) {
    match stored {
        Some(grants) => (PeerResolution::Stale(reason), grants),
        None => (PeerResolution::Unreachable(reason), empty()),
    }
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
            return forget(link, PeerResolution::Unknown, ports).await
        }
        PeerGrantAnswer::Invalid(reason) => {
            return forget(link, PeerResolution::Rejected(reason), ports).await
        }
        PeerGrantAnswer::Verified(assertion) => assertion,
    };

    if let Some(reason) = guard(peer, &assertion, ports).await? {
        return forget(link, PeerResolution::Rejected(reason), ports).await;
    }

    let grants = assertion.payload.grants.clone();
    let latest = Local::now() + limits.assertion_ttl;
    match store(link, assertion.kept_until(latest), ports).await? {
        true => Ok((PeerResolution::Answered, grants)),
        false => Ok((PeerResolution::Unknown, empty())),
    }
}

/// Drop the link's stored assertion: the peer no longer vouches for it
async fn forget(
    link: &IdentityLink,
    resolution: PeerResolution,
    ports: &ResolutionPorts<'_>,
) -> Result<(PeerResolution, FederatedGrants), MappedErrors> {
    ports.store.write.forget(link.id).await?;
    Ok((resolution, empty()))
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
) -> Result<bool, MappedErrors> {
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
    let failed =
        outcome.resolution.is_partial() || outcome.resolution.is_stale();
    let contact = match failed {
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

fn empty() -> FederatedGrants {
    FederatedGrants::default()
}
