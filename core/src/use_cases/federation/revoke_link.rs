use crate::domain::{
    dtos::{
        federation::{IdentityLink, LinkRevocation},
        written_by::WrittenBy,
    },
    entities::{
        FederationPeerRead, IdentityLinkRead, IdentityLinkWrite,
        PeerLinkNotifier,
    },
};

use mycelium_base::{
    entities::{FetchResponseKind, UpdatingResponseKind},
    utils::errors::{use_case_err, MappedErrors},
};
use uuid::Uuid;

/// Who asks to revoke a link
pub enum LinkRevoker {
    /// The linked user: only their own links
    User(Uuid),
    /// An operator (FED-34 guard checked by the caller)
    Operator,
}

/// Revoke a link here and tell the peer (FED-47)
///
/// The local revocation stands whatever the peer answers: a failed
/// notification is logged, and the durable retry is the tombstone outbox of
/// a later phase.
///
#[tracing::instrument(name = "revoke_link", skip_all)]
pub async fn revoke_link(
    link_id: Uuid,
    revoker: LinkRevoker,
    revoked_by: Option<WrittenBy>,
    link_read_repo: Box<&dyn IdentityLinkRead>,
    link_write_repo: Box<&dyn IdentityLinkWrite>,
    peer_read_repo: Box<&dyn FederationPeerRead>,
    notifier: Box<&dyn PeerLinkNotifier>,
) -> Result<IdentityLink, MappedErrors> {
    let link = own_link(link_id, revoker, *link_read_repo).await?;

    let UpdatingResponseKind::Updated(revoked) =
        link_write_repo.revoke(link.id, revoked_by).await?
    else {
        return refused("the link is already revoked");
    };

    notify_peer(&revoked, *peer_read_repo, *notifier).await;

    Ok(revoked)
}

async fn own_link(
    link_id: Uuid,
    revoker: LinkRevoker,
    link_read_repo: &dyn IdentityLinkRead,
) -> Result<IdentityLink, MappedErrors> {
    let FetchResponseKind::Found(link) = link_read_repo.get(link_id).await?
    else {
        return refused("unknown link");
    };

    match revoker {
        LinkRevoker::User(user_id) if link.local_user_id != user_id => {
            refused("unknown link")
        }
        _ => Ok(link),
    }
}

async fn notify_peer(
    link: &IdentityLink,
    peer_read_repo: &dyn FederationPeerRead,
    notifier: &dyn PeerLinkNotifier,
) {
    let (Some(subject_ref), Ok(FetchResponseKind::Found(peer))) = (
        link.peer_subject_ref.clone(),
        peer_read_repo.get(link.peer_id).await,
    ) else {
        return;
    };

    if let Err(err) = notifier
        .revoke_link(&peer, LinkRevocation { subject_ref })
        .await
    {
        tracing::warn!(
            %err,
            peer = %peer.instance_id,
            "the peer was not told about a revoked link"
        );
    }
}

fn refused<T>(reason: &str) -> Result<T, MappedErrors> {
    use_case_err(reason).with_exp_true().as_error()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::dtos::federation::{
            LinkStatus, NewIdentityLink, PeerStatus, SubjectRef,
        },
        use_cases::federation::link_test_doubles::{
            peer, LinkStore, PeerNotifier,
        },
    };

    use mycelium_base::entities::CreateResponseKind;

    async fn active_link(store: &LinkStore, user_id: Uuid) -> IdentityLink {
        let CreateResponseKind::Created(link) = store
            .record(NewIdentityLink {
                local_user_id: user_id,
                local_email: "user@example.com".to_string(),
                peer_id: store.peers[0].id,
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

    async fn revoke(
        store: &LinkStore,
        notifier: &PeerNotifier,
        link_id: Uuid,
        revoker: LinkRevoker,
    ) -> Result<IdentityLink, MappedErrors> {
        revoke_link(
            link_id,
            revoker,
            None,
            Box::new(store),
            Box::new(store),
            Box::new(store),
            Box::new(notifier),
        )
        .await
    }

    #[tokio::test]
    async fn a_user_revokes_their_link_and_the_peer_is_told() {
        let store = LinkStore::with_peers(vec![peer(PeerStatus::Active)]);
        let notifier = PeerNotifier::default();
        let user_id = Uuid::new_v4();
        let link = active_link(&store, user_id).await;

        let revoked =
            revoke(&store, &notifier, link.id, LinkRevoker::User(user_id))
                .await
                .unwrap();

        assert_eq!(revoked.status, LinkStatus::Revoked);
        assert_eq!(
            notifier.revoked.lock().unwrap()[0].subject_ref,
            link.peer_subject_ref.unwrap()
        );
    }

    #[tokio::test]
    async fn a_user_cannot_revoke_another_users_link() {
        let store = LinkStore::with_peers(vec![peer(PeerStatus::Active)]);
        let notifier = PeerNotifier::default();
        let link = active_link(&store, Uuid::new_v4()).await;

        let outcome = revoke(
            &store,
            &notifier,
            link.id,
            LinkRevoker::User(Uuid::new_v4()),
        )
        .await;

        assert!(outcome.is_err());
        assert_eq!(store.links()[0].status, LinkStatus::Active);
    }

    #[tokio::test]
    async fn an_unreachable_peer_keeps_the_local_revocation() {
        let store = LinkStore::with_peers(vec![peer(PeerStatus::Active)]);
        let notifier = PeerNotifier {
            refuses: true,
            ..PeerNotifier::default()
        };
        let link = active_link(&store, Uuid::new_v4()).await;

        let revoked = revoke(&store, &notifier, link.id, LinkRevoker::Operator)
            .await
            .unwrap();

        assert_eq!(revoked.status, LinkStatus::Revoked);
    }
}
