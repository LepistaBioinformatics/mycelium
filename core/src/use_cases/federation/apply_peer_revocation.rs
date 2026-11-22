use crate::domain::{
    dtos::federation::{FederationPeer, LinkRevocation},
    entities::{IdentityLinkRead, IdentityLinkWrite},
};

use mycelium_base::{entities::FetchResponseKind, utils::errors::MappedErrors};

/// A peer revoked a link on its side: revoke this side's copy (FED-47)
///
/// `peer` is the verified caller; only its own links are reachable. An
/// unknown or already revoked reference is not an error, so the answer never
/// tells a peer which references exist.
///
#[tracing::instrument(name = "apply_peer_revocation", skip_all)]
pub async fn apply_peer_revocation(
    peer: &FederationPeer,
    revocation: LinkRevocation,
    link_read_repo: Box<&dyn IdentityLinkRead>,
    link_write_repo: Box<&dyn IdentityLinkWrite>,
) -> Result<(), MappedErrors> {
    let FetchResponseKind::Found(link) = link_read_repo
        .find_by_local_ref(peer.id, revocation.subject_ref)
        .await?
    else {
        return Ok(());
    };

    link_write_repo.revoke(link.id, None).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::dtos::federation::{
            LinkStatus, NewIdentityLink, PeerStatus, SubjectRef,
        },
        use_cases::federation::link_test_doubles::{peer, LinkStore},
    };

    use mycelium_base::entities::CreateResponseKind;
    use uuid::Uuid;

    #[tokio::test]
    async fn only_the_callers_link_is_revoked_and_unknown_refs_pass() {
        let first = peer(PeerStatus::Active);
        let second = peer(PeerStatus::Active);
        let store = LinkStore::with_peers(vec![first.clone(), second.clone()]);
        let CreateResponseKind::Created(link) = store
            .record(NewIdentityLink {
                local_user_id: Uuid::new_v4(),
                local_email: "user@example.com".to_string(),
                peer_id: first.id,
                peer_subject_ref: Some(SubjectRef::mint()),
                status: LinkStatus::Active,
            })
            .await
            .unwrap()
        else {
            panic!("the link must be recorded");
        };
        let revocation = LinkRevocation {
            subject_ref: link.local_subject_ref.clone(),
        };

        apply_peer_revocation(
            &second,
            revocation.clone(),
            Box::new(&store),
            Box::new(&store),
        )
        .await
        .unwrap();
        assert_eq!(store.links()[0].status, LinkStatus::Active);

        apply_peer_revocation(
            &first,
            revocation,
            Box::new(&store),
            Box::new(&store),
        )
        .await
        .unwrap();
        assert_eq!(store.links()[0].status, LinkStatus::Revoked);
    }
}
