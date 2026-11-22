use super::{refresh_peer_grants, ResolutionLimits, ResolutionPorts};
use crate::domain::dtos::{
    federation::{
        FederatedGrants, FederatedResolution, FederationPeer, IdentityLink,
        LinkStatus, PeerOutcome, PeerResolution, PeerStatus, StoredAssertion,
        PROFILE_ASSERTION_KIND,
    },
    written_by::WrittenBy,
};

use chrono::Local;
use futures::future::join_all;
use mycelium_base::{entities::FetchResponseKind, utils::errors::MappedErrors};
use uuid::Uuid;

/// A peer to ask, with the grants of its unexpired stored assertion, if any
type Ask = (IdentityLink, FederationPeer, Option<FederatedGrants>);

/// What to do for one linked peer
enum Step {
    Done(PeerOutcome),
    Ask(Box<Ask>),
}

/// The federated part of a local user's grants (FED-11, FED-15, FED-17,
/// FED-18, FED-53)
///
/// Only the peers the user has an active link with are considered, and only
/// while they are trusted, `profile.v1` was negotiated and their policy lets
/// them assert. A stored assertion younger than `assertion_refresh` is used
/// as is. An older one, or none, makes the peer be asked, in parallel with
/// the others under the deadline; if the peer does not answer, its unexpired
/// assertion is served as stale, and an expired one contributes nothing. A
/// peer whose circuit is open is not called: its unexpired assertion is
/// served as stale. The caller must already hold the user's local profile: a
/// subject without a local account never reaches this point.
///
#[tracing::instrument(name = "resolve_federated_grants", skip_all)]
pub async fn resolve_federated_grants(
    local_user_id: Uuid,
    author: &WrittenBy,
    limits: ResolutionLimits,
    ports: &ResolutionPorts<'_>,
) -> Result<FederatedResolution, MappedErrors> {
    let links = active_links(local_user_id, ports).await?;
    let link_ids: Vec<Uuid> = links.iter().map(|link| link.id).collect();
    let stored = ports.store.read.list_valid_for_links(&link_ids).await?;

    let mut outcomes = vec![];
    let mut asks = vec![];
    for link in links {
        match step_for(link, &stored, limits, ports).await? {
            Step::Done(outcome) => outcomes.push(outcome),
            Step::Ask(ask) => asks.push(*ask),
        }
    }

    for refreshed in join_all(asks.into_iter().map(|(link, peer, stored)| {
        refresh_peer_grants(link, peer, stored, author, limits, ports)
    }))
    .await
    {
        outcomes.push(refreshed?);
    }

    Ok(FederatedResolution(outcomes))
}

async fn active_links(
    local_user_id: Uuid,
    ports: &ResolutionPorts<'_>,
) -> Result<Vec<IdentityLink>, MappedErrors> {
    let links = ports.links.list(Some(local_user_id)).await?;

    Ok(links
        .into_iter()
        .filter(|link| link.status == LinkStatus::Active)
        .filter(|link| link.peer_subject_ref.is_some())
        .collect())
}

async fn step_for(
    link: IdentityLink,
    stored: &[StoredAssertion],
    limits: ResolutionLimits,
    ports: &ResolutionPorts<'_>,
) -> Result<Step, MappedErrors> {
    let FetchResponseKind::Found(peer) =
        ports.peers.read.get(link.peer_id).await?
    else {
        return Ok(Step::Done(skipped_unknown_peer(&link)));
    };

    if let Some(reason) = excluded(&peer) {
        return Ok(done(&peer, PeerResolution::Skipped(reason), None));
    }

    let found = stored.iter().find(|stored| stored.link_id == link.id);
    if !matches!(peer.status, PeerStatus::Active | PeerStatus::Degraded) {
        return Ok(circuit_open(&peer, found));
    }

    match found {
        Some(found)
            if !found
                .assertion
                .is_due(Local::now(), limits.assertion_refresh) =>
        {
            Ok(done(&peer, PeerResolution::Stored, Some(found)))
        }
        _ => {
            let grants =
                found.map(|found| found.assertion.payload.grants.clone());
            Ok(Step::Ask(Box::new((link, peer, grants))))
        }
    }
}

/// Why the peer must never contribute, if it must not: an operator or
/// handshake decision, or nothing negotiated or allowed to assert
fn excluded(peer: &FederationPeer) -> Option<String> {
    if matches!(peer.status, PeerStatus::Quarantined | PeerStatus::Untrusted) {
        return Some(format!("peer is {}", peer.status.as_str()));
    }
    if !peer
        .negotiated_kinds
        .iter()
        .any(|kind| kind == PROFILE_ASSERTION_KIND)
    {
        return Some(format!("{PROFILE_ASSERTION_KIND} not negotiated"));
    }
    if !peer.policy.may_assert {
        return Some("peer policy does not allow assertions".to_string());
    }

    None
}

/// A peer that must not be called now serves its unexpired assertion as
/// stale (FED-17), or nothing (FED-18)
fn circuit_open(
    peer: &FederationPeer,
    found: Option<&StoredAssertion>,
) -> Step {
    let reason = format!("peer is {}", peer.status.as_str());

    match found {
        Some(found) => done(peer, PeerResolution::Stale(reason), Some(found)),
        None => done(peer, PeerResolution::Skipped(reason), None),
    }
}

fn done(
    peer: &FederationPeer,
    resolution: PeerResolution,
    found: Option<&StoredAssertion>,
) -> Step {
    let grants = found
        .map(|found| found.assertion.payload.grants.clone())
        .unwrap_or_default();

    Step::Done(outcome(peer, resolution, grants))
}

fn outcome(
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

fn skipped_unknown_peer(link: &IdentityLink) -> PeerOutcome {
    PeerOutcome {
        peer_id: link.peer_id,
        instance_id: Uuid::nil(),
        resolution: PeerResolution::Skipped("unknown peer".to_string()),
        grants: FederatedGrants::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::dtos::{
            federation::{
                PeerGrantAnswer, ProfileAssertion, SubjectRef,
                VerifiedAssertion,
            },
            guest_role::Permission,
            profile::LicensedResource,
        },
        use_cases::federation::{
            link_test_doubles::{peer, LinkStore},
            resolution_test_doubles::{Mesh, Script},
            ResolutionGuardPorts, ResolutionPeerPorts, ResolutionStorePorts,
        },
    };

    use chrono::{Duration as Lifetime, Local};
    use std::time::Duration;

    fn trusted_peer() -> FederationPeer {
        let mut trusted = peer(PeerStatus::Active);
        trusted.negotiated_kinds = vec![PROFILE_ASSERTION_KIND.to_string()];
        trusted.policy.may_assert = true;
        trusted
    }

    fn link_with(user_id: Uuid, peer: &FederationPeer) -> IdentityLink {
        IdentityLink {
            id: Uuid::new_v4(),
            local_user_id: user_id,
            local_email: "user-01@example.com".to_string(),
            peer_id: peer.id,
            peer_subject_ref: Some(SubjectRef::mint()),
            local_subject_ref: SubjectRef::mint(),
            status: LinkStatus::Active,
            created: Local::now(),
            established_at: None,
            revoked_at: None,
            revoked_by: None,
        }
    }

    fn verified(tenant_id: Uuid) -> PeerGrantAnswer {
        let now = Local::now();
        PeerGrantAnswer::Verified(VerifiedAssertion {
            jws: "jws".to_string(),
            payload: ProfileAssertion::from_grants(FederatedGrants(vec![
                LicensedResource {
                    acc_id: Uuid::new_v4(),
                    sys_acc: false,
                    tenant_id,
                    acc_name: "account-01".to_string(),
                    role: "reader".to_string(),
                    role_id: Uuid::new_v4(),
                    perm: Permission::Read,
                    verified: true,
                    permit_flags: None,
                    deny_flags: None,
                },
            ])),
            issued_at: now,
            expires_at: now + Lifetime::seconds(60),
        })
    }

    struct World {
        user_id: Uuid,
        links: LinkStore,
        mesh: Mesh,
    }

    /// One link of a fresh user with each peer
    fn world(peers: Vec<FederationPeer>, mesh: Mesh) -> World {
        let user_id = Uuid::new_v4();
        let links = LinkStore::with_peers(peers.clone());
        links
            .links
            .lock()
            .unwrap()
            .extend(peers.iter().map(|peer| link_with(user_id, peer)));

        World {
            user_id,
            links,
            mesh,
        }
    }

    fn ports_of(world: &World) -> ResolutionPorts<'_> {
        ResolutionPorts {
            links: &world.links,
            peers: ResolutionPeerPorts {
                read: &world.links,
                write: &world.mesh,
                resolver: &world.mesh,
            },
            store: ResolutionStorePorts {
                read: &world.mesh,
                write: &world.mesh,
            },
            guard: ResolutionGuardPorts {
                probe: &world.mesh,
                claims: &world.mesh,
            },
            audit: &world.mesh,
        }
    }

    fn limits() -> ResolutionLimits {
        ResolutionLimits {
            deadline: Duration::from_millis(50),
            max_failures_before_open: 5,
            assertion_ttl: Duration::from_secs(60),
            assertion_refresh: Duration::from_secs(30),
        }
    }

    async fn resolve(world: &World) -> FederatedResolution {
        resolve_federated_grants(
            world.user_id,
            &WrittenBy::default(),
            limits(),
            &ports_of(world),
        )
        .await
        .unwrap()
    }

    fn only(resolution: &FederatedResolution) -> &PeerOutcome {
        assert_eq!(resolution.0.len(), 1);
        &resolution.0[0]
    }

    #[tokio::test]
    async fn an_answer_is_stored_and_the_next_resolution_reads_the_store() {
        let issuer = trusted_peer();
        let mut mesh = Mesh::default();
        mesh.scripts
            .insert(issuer.id, Script::Answer(verified(Uuid::new_v4())));
        let world = world(vec![issuer], mesh);

        let first = resolve(&world).await;
        let second = resolve(&world).await;

        assert_eq!(only(&first).resolution, PeerResolution::Answered);
        assert_eq!(only(&first).grants.len(), 1);
        assert_eq!(only(&second).resolution, PeerResolution::Stored);
        assert_eq!(only(&second).grants, only(&first).grants);
        assert_eq!(world.mesh.asked.lock().unwrap().len(), 1);
        assert_eq!(world.mesh.audited.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn a_hanging_or_failing_peer_gives_a_partial_result() {
        let (hanging, failing) = (trusted_peer(), trusted_peer());
        let mut mesh = Mesh::default();
        mesh.scripts.insert(hanging.id, Script::Hang);
        mesh.scripts.insert(failing.id, Script::Fail);
        let world = world(vec![hanging, failing], mesh);

        let resolution = resolve(&world).await;

        assert!(resolution.is_partial());
        assert!(resolution.0.iter().all(|outcome| matches!(
            outcome.resolution,
            PeerResolution::Unreachable(_)
        )));
        assert_eq!(world.mesh.failures.lock().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn a_colliding_id_is_rejected_counted_and_not_stored() {
        let issuer = trusted_peer();
        let local_tenant = Uuid::new_v4();
        let mut mesh = Mesh::default();
        mesh.local_ids = vec![local_tenant];
        mesh.scripts
            .insert(issuer.id, Script::Answer(verified(local_tenant)));
        let world = world(vec![issuer], mesh);

        let resolution = resolve(&world).await;

        assert!(matches!(
            only(&resolution).resolution,
            PeerResolution::Rejected(_)
        ));
        assert!(only(&resolution).grants.is_empty());
        assert!(world.mesh.stored.lock().unwrap().is_empty());
        assert_eq!(world.mesh.failures.lock().unwrap().len(), 1);
        assert_eq!(world.mesh.audited.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn untrusted_or_unconfigured_peers_are_never_asked() {
        let quarantined = {
            let mut peer = trusted_peer();
            peer.status = PeerStatus::Quarantined;
            peer
        };
        let silent = {
            let mut peer = trusted_peer();
            peer.policy.may_assert = false;
            peer
        };
        let world = world(vec![quarantined, silent], Mesh::default());

        let resolution = resolve(&world).await;

        assert!(resolution.0.iter().all(|outcome| matches!(
            outcome.resolution,
            PeerResolution::Skipped(_)
        )));
        assert!(world.mesh.asked.lock().unwrap().is_empty());
        assert!(!resolution.is_partial());
    }

    /// A stored assertion of the world's only link, issued `age` seconds ago
    /// and valid for 60 seconds from then
    fn stored_aged(world: &World, age: i64) -> FederatedGrants {
        let link = world.links.links()[0].clone();
        let PeerGrantAnswer::Verified(mut assertion) = verified(Uuid::new_v4())
        else {
            unreachable!()
        };
        assertion.issued_at = Local::now() - Lifetime::seconds(age);
        assertion.expires_at = assertion.issued_at + Lifetime::seconds(60);
        let grants = assertion.payload.grants.clone();
        world.mesh.stored.lock().unwrap().push(StoredAssertion {
            link_id: link.id,
            peer_id: link.peer_id,
            assertion,
        });
        grants
    }

    #[tokio::test]
    async fn a_due_assertion_is_refreshed_when_the_peer_answers() {
        let issuer = trusted_peer();
        let mut mesh = Mesh::default();
        mesh.scripts
            .insert(issuer.id, Script::Answer(verified(Uuid::new_v4())));
        let world = world(vec![issuer], mesh);
        let old = stored_aged(&world, 40);

        let resolution = resolve(&world).await;

        assert_eq!(only(&resolution).resolution, PeerResolution::Answered);
        assert_ne!(only(&resolution).grants, old);
        assert_eq!(world.mesh.asked.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn a_due_assertion_is_served_stale_when_the_peer_is_silent() {
        let issuer = trusted_peer();
        let mut mesh = Mesh::default();
        mesh.scripts.insert(issuer.id, Script::Hang);
        let world = world(vec![issuer], mesh);
        let old = stored_aged(&world, 40);

        let resolution = resolve(&world).await;

        assert!(matches!(
            only(&resolution).resolution,
            PeerResolution::Stale(_)
        ));
        assert_eq!(only(&resolution).grants, old);
        assert!(resolution.is_stale());
        assert!(!resolution.is_partial());
        assert_eq!(world.mesh.failures.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn an_open_circuit_serves_the_stored_assertion_stale_without_a_call()
    {
        let unreachable_peer = {
            let mut peer = trusted_peer();
            peer.status = PeerStatus::Unreachable;
            peer
        };
        let world = world(vec![unreachable_peer], Mesh::default());

        let without = resolve(&world).await;
        let old = stored_aged(&world, 5);
        let with = resolve(&world).await;

        assert!(matches!(
            only(&without).resolution,
            PeerResolution::Skipped(_)
        ));
        assert!(matches!(only(&with).resolution, PeerResolution::Stale(_)));
        assert_eq!(only(&with).grants, old);
        assert!(world.mesh.asked.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn an_unknown_answer_forgets_the_stored_assertion() {
        let issuer = trusted_peer();
        let mut mesh = Mesh::default();
        mesh.scripts
            .insert(issuer.id, Script::Answer(PeerGrantAnswer::Unknown));
        let world = world(vec![issuer], mesh);
        stored_aged(&world, 40);

        let resolution = resolve(&world).await;

        assert_eq!(only(&resolution).resolution, PeerResolution::Unknown);
        assert!(only(&resolution).grants.is_empty());
        assert!(world.mesh.stored.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn an_answer_issued_before_the_links_tombstone_is_not_stored() {
        let issuer = trusted_peer();
        let mut mesh = Mesh::default();
        mesh.scripts
            .insert(issuer.id, Script::Answer(verified(Uuid::new_v4())));
        let world = world(vec![issuer], mesh);
        let link_id = world.links.links()[0].id;
        world.mesh.superseded.lock().unwrap().push(link_id);

        let resolution = resolve(&world).await;

        assert_eq!(only(&resolution).resolution, PeerResolution::Unknown);
        assert!(only(&resolution).grants.is_empty());
        assert!(world.mesh.stored.lock().unwrap().is_empty());
    }
}
