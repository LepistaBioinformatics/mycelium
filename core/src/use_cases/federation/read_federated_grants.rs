use super::peer_excluded_from_assertions::excluded;
use crate::domain::{
    dtos::{
        federation::{
            IdentityLink, LinkStatus, StoredAssertion, StoredFederatedGrants,
        },
        profile::FederatedResource,
    },
    entities::{AssertionStoreRead, FederationPeerRead, IdentityLinkRead},
};

use chrono::Local;
use mycelium_base::{entities::FetchResponseKind, utils::errors::MappedErrors};
use std::time::Duration;
use uuid::Uuid;

/// A subject's federated grants from the store alone, for the request path
/// (FED-11, FED-26)
///
/// Never calls a peer. Only active links count, and only peers still trusted
/// to assert; every unexpired stored assertion contributes its grants with
/// the peer's instance id as issuer. A subject without links costs one read.
/// `refresh_due` says some such peer has nothing stored or a due assertion.
///
#[tracing::instrument(name = "read_federated_grants", skip_all)]
pub async fn read_federated_grants(
    local_user_id: Uuid,
    refresh_after: Duration,
    link_read_repo: Box<&dyn IdentityLinkRead>,
    peer_read_repo: Box<&dyn FederationPeerRead>,
    store_read_repo: Box<&dyn AssertionStoreRead>,
) -> Result<StoredFederatedGrants, MappedErrors> {
    let links = active_links(local_user_id, *link_read_repo).await?;
    if links.is_empty() {
        return Ok(StoredFederatedGrants::default());
    }

    let ids: Vec<Uuid> = links.iter().map(|link| link.id).collect();
    let stored = store_read_repo.list_valid_for_links(&ids).await?;
    let mut read = StoredFederatedGrants::default();

    for link in links {
        add_link(&mut read, &link, &stored, refresh_after, *peer_read_repo)
            .await?;
    }

    Ok(read)
}

async fn active_links(
    local_user_id: Uuid,
    link_read_repo: &dyn IdentityLinkRead,
) -> Result<Vec<IdentityLink>, MappedErrors> {
    let links = link_read_repo.list(Some(local_user_id)).await?;

    Ok(links
        .into_iter()
        .filter(|link| link.status == LinkStatus::Active)
        .filter(|link| link.peer_subject_ref.is_some())
        .collect())
}

/// Add the link's stored grants, and whether it needs a refresh
async fn add_link(
    read: &mut StoredFederatedGrants,
    link: &IdentityLink,
    stored: &[StoredAssertion],
    refresh_after: Duration,
    peer_read_repo: &dyn FederationPeerRead,
) -> Result<(), MappedErrors> {
    let FetchResponseKind::Found(peer) =
        peer_read_repo.get(link.peer_id).await?
    else {
        return Ok(());
    };
    if excluded(&peer).is_some() {
        return Ok(());
    }

    let Some(found) = stored.iter().find(|stored| stored.link_id == link.id)
    else {
        read.refresh_due = true;
        return Ok(());
    };

    read.refresh_due |= found.assertion.is_due(Local::now(), refresh_after);
    read.resources
        .0
        .extend(found.assertion.payload.grants.0.iter().map(|grant| {
            FederatedResource {
                issuer: peer.instance_id,
                grant: grant.clone(),
            }
        }));

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::dtos::federation::{
            FederatedGrants, FederationPolicy, PeerStatus, ProfileAssertion,
            SubjectRef, VerifiedAssertion, PROFILE_ASSERTION_KIND,
        },
        use_cases::federation::{
            link_test_doubles::{peer, LinkStore},
            resolution_test_doubles::Mesh,
        },
    };

    use crate::domain::dtos::{
        guest_role::Permission, profile::LicensedResource,
    };
    use chrono::Duration as ChronoDuration;

    fn trusted(
        status: PeerStatus,
    ) -> crate::domain::dtos::federation::FederationPeer {
        let mut trusted = peer(status);
        trusted.negotiated_kinds = vec![PROFILE_ASSERTION_KIND.to_string()];
        trusted.policy = FederationPolicy {
            may_assert: true,
            ..FederationPolicy::default()
        };
        trusted
    }

    fn link_with(user_id: Uuid, peer_id: Uuid) -> IdentityLink {
        IdentityLink {
            id: Uuid::new_v4(),
            local_user_id: user_id,
            local_email: "user-01@example.com".to_string(),
            peer_id,
            peer_subject_ref: Some(SubjectRef::mint()),
            local_subject_ref: SubjectRef::mint(),
            status: LinkStatus::Active,
            created: Local::now(),
            established_at: None,
            revoked_at: None,
            revoked_by: None,
        }
    }

    fn assertion(link: &IdentityLink, age: i64) -> StoredAssertion {
        let issued_at = Local::now() - ChronoDuration::seconds(age);
        StoredAssertion {
            link_id: link.id,
            peer_id: link.peer_id,
            assertion: VerifiedAssertion {
                jws: "jws".to_string(),
                payload: ProfileAssertion::from_grants(FederatedGrants(vec![
                    LicensedResource {
                        acc_id: Uuid::new_v4(),
                        sys_acc: false,
                        tenant_id: Uuid::new_v4(),
                        acc_name: "account-01".to_string(),
                        role: "reader".to_string(),
                        role_id: Uuid::new_v4(),
                        perm: Permission::Read,
                        verified: true,
                        permit_flags: None,
                        deny_flags: None,
                    },
                ])),
                issued_at,
                expires_at: issued_at + ChronoDuration::seconds(60),
            },
        }
    }

    async fn read(
        user_id: Uuid,
        store: &LinkStore,
        mesh: &Mesh,
    ) -> StoredFederatedGrants {
        read_federated_grants(
            user_id,
            Duration::from_secs(30),
            Box::new(store),
            Box::new(store),
            Box::new(mesh),
        )
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn a_subject_without_links_reads_nothing() {
        let read =
            read(Uuid::new_v4(), &LinkStore::default(), &Mesh::default()).await;

        assert_eq!(read, StoredFederatedGrants::default());
    }

    #[tokio::test]
    async fn stored_grants_carry_their_issuer_and_untrusted_peers_none() {
        let user_id = Uuid::new_v4();
        let (issuer, quarantined) = (
            trusted(PeerStatus::Unreachable),
            trusted(PeerStatus::Quarantined),
        );
        let store =
            LinkStore::with_peers(vec![issuer.clone(), quarantined.clone()]);
        let (served, refused) = (
            link_with(user_id, issuer.id),
            link_with(user_id, quarantined.id),
        );
        store
            .links
            .lock()
            .unwrap()
            .extend([served.clone(), refused.clone()]);
        let mesh = Mesh::default();
        mesh.stored
            .lock()
            .unwrap()
            .extend([assertion(&served, 5), assertion(&refused, 5)]);

        let read = read(user_id, &store, &mesh).await;

        assert_eq!(read.resources.0.len(), 1);
        assert_eq!(read.resources.0[0].issuer, issuer.instance_id);
        assert!(!read.refresh_due);
    }

    #[tokio::test]
    async fn a_missing_or_due_assertion_asks_for_a_refresh() {
        let user_id = Uuid::new_v4();
        let issuer = trusted(PeerStatus::Active);
        let store = LinkStore::with_peers(vec![issuer.clone()]);
        let due = link_with(user_id, issuer.id);
        store.links.lock().unwrap().push(due.clone());
        let mesh = Mesh::default();

        assert!(read(user_id, &store, &mesh).await.refresh_due);

        mesh.stored.lock().unwrap().push(assertion(&due, 40));
        let read = read(user_id, &store, &mesh).await;
        assert!(read.refresh_due);
        assert_eq!(read.resources.0.len(), 1);
    }
}
