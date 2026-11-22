use crate::domain::{
    dtos::federation::{
        FederationPeer, IdentityLink, LinkStatus, Tombstone, TombstoneReason,
    },
    entities::{IdentityLinkRead, IdentityLinkWrite, TombstoneRegistration},
};

use mycelium_base::{entities::FetchResponseKind, utils::errors::MappedErrors};

/// Apply a tombstone a peer pushed (FED-23, FED-47)
///
/// Only the sender's own links are touched, and an unknown reference changes
/// nothing. The tombstone is recorded durably and the link's stored
/// assertion dropped, at once on every pod; an assertion not issued after it
/// is never stored again. `linkRevoked` also revokes this side of the link.
///
#[tracing::instrument(name = "apply_tombstone", skip_all)]
pub async fn apply_tombstone(
    peer: &FederationPeer,
    tombstone: Tombstone,
    link_read_repo: Box<&dyn IdentityLinkRead>,
    link_write_repo: Box<&dyn IdentityLinkWrite>,
    registration: Box<&dyn TombstoneRegistration>,
) -> Result<(), MappedErrors> {
    let FetchResponseKind::Found(link) = link_read_repo
        .find_by_local_ref(peer.id, tombstone.subject_ref.clone())
        .await?
    else {
        return Ok(());
    };

    registration.record(link.id, peer.id, &tombstone).await?;

    if unlinks(&link, &tombstone) {
        link_write_repo.revoke(link.id, None).await?;
    }

    Ok(())
}

fn unlinks(link: &IdentityLink, tombstone: &Tombstone) -> bool {
    tombstone.reason == TombstoneReason::LinkRevoked
        && link.status != LinkStatus::Revoked
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::dtos::federation::{NewIdentityLink, PeerStatus, SubjectRef},
        use_cases::federation::{
            link_test_doubles::{peer, LinkStore},
            revocation_test_doubles::Revocations,
        },
    };

    use chrono::Local;
    use mycelium_base::entities::CreateResponseKind;
    use uuid::Uuid;

    async fn active_link(store: &LinkStore, peer_id: Uuid) -> IdentityLink {
        let CreateResponseKind::Created(link) = store
            .record(NewIdentityLink {
                local_user_id: Uuid::new_v4(),
                local_email: "user-01@example.com".to_string(),
                peer_id,
                peer_subject_ref: Some(SubjectRef::mint()),
                status: LinkStatus::Active,
            })
            .await
            .unwrap()
        else {
            panic!("the link must be recorded");
        };

        link
    }

    fn tombstone(link: &IdentityLink, reason: TombstoneReason) -> Tombstone {
        Tombstone {
            subject_ref: link.local_subject_ref.clone(),
            reason,
            issued_at: Local::now(),
        }
    }

    async fn apply(
        sender: &FederationPeer,
        tombstone: Tombstone,
        store: &LinkStore,
        revocations: &Revocations,
    ) {
        apply_tombstone(
            sender,
            tombstone,
            Box::new(store),
            Box::new(store),
            Box::new(revocations),
        )
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn a_grants_tombstone_is_recorded_and_keeps_the_link() {
        let sender = peer(PeerStatus::Active);
        let store = LinkStore::with_peers(vec![sender.clone()]);
        let revocations = Revocations::default();
        let link = active_link(&store, sender.id).await;

        let received = tombstone(&link, TombstoneReason::GrantsChanged);
        apply(&sender, received, &store, &revocations).await;

        assert_eq!(revocations.recorded.lock().unwrap()[0].0, link.id);
        assert_eq!(store.links()[0].status, LinkStatus::Active);
    }

    #[tokio::test]
    async fn a_link_tombstone_unlinks_and_only_the_senders_links_are_touched() {
        let (sender, other) =
            (peer(PeerStatus::Active), peer(PeerStatus::Active));
        let store = LinkStore::with_peers(vec![sender.clone(), other.clone()]);
        let revocations = Revocations::default();
        let link = active_link(&store, sender.id).await;
        let received = tombstone(&link, TombstoneReason::LinkRevoked);

        apply(&other, received.clone(), &store, &revocations).await;
        assert!(revocations.recorded.lock().unwrap().is_empty());
        assert_eq!(store.links()[0].status, LinkStatus::Active);

        apply(&sender, received, &store, &revocations).await;
        assert_eq!(store.links()[0].status, LinkStatus::Revoked);
    }
}
