use super::queue_revocations::queue_for_links;
use crate::domain::{
    dtos::{
        federation::{IdentityLink, TombstoneReason},
        written_by::WrittenBy,
    },
    entities::{IdentityLinkRead, IdentityLinkWrite, RevocationOutbox},
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

/// Revoke a link here and queue a tombstone for the peer (FED-47)
///
/// The outbox delivers it with retry (FED-24); the local revocation stands
/// whatever the peer answers.
///
#[tracing::instrument(name = "revoke_link", skip_all)]
pub async fn revoke_link(
    link_id: Uuid,
    revoker: LinkRevoker,
    revoked_by: Option<WrittenBy>,
    link_read_repo: Box<&dyn IdentityLinkRead>,
    link_write_repo: Box<&dyn IdentityLinkWrite>,
    outbox: Box<&dyn RevocationOutbox>,
) -> Result<IdentityLink, MappedErrors> {
    let link = own_link(link_id, revoker, *link_read_repo).await?;

    let UpdatingResponseKind::Updated(revoked) =
        link_write_repo.revoke(link.id, revoked_by).await?
    else {
        return refused("the link is already revoked");
    };

    tell_peer(&revoked, *outbox).await;

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

/// Queue a `linkRevoked` tombstone for the peer (FED-47); the local
/// revocation stands even when the queue cannot be written
async fn tell_peer(link: &IdentityLink, outbox: &dyn RevocationOutbox) {
    let reason = TombstoneReason::LinkRevoked;

    if let Err(err) =
        queue_for_links(std::slice::from_ref(link), reason, outbox).await
    {
        tracing::error!(%err, link = %link.id, "a revoked link was not queued");
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
        use_cases::federation::{
            link_test_doubles::{peer, LinkStore},
            revocation_test_doubles::Revocations,
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
        outbox: &Revocations,
        link_id: Uuid,
        revoker: LinkRevoker,
    ) -> Result<IdentityLink, MappedErrors> {
        revoke_link(
            link_id,
            revoker,
            None,
            Box::new(store),
            Box::new(store),
            Box::new(outbox),
        )
        .await
    }

    #[tokio::test]
    async fn a_user_revokes_their_link_and_a_tombstone_is_queued() {
        let store = LinkStore::with_peers(vec![peer(PeerStatus::Active)]);
        let outbox = Revocations::default();
        let user_id = Uuid::new_v4();
        let link = active_link(&store, user_id).await;

        let revoked =
            revoke(&store, &outbox, link.id, LinkRevoker::User(user_id))
                .await
                .unwrap();

        assert_eq!(revoked.status, LinkStatus::Revoked);
        let queued = outbox.queued.lock().unwrap();
        assert_eq!(queued[0].peer_id, link.peer_id);
        assert_eq!(queued[0].tombstone.reason, TombstoneReason::LinkRevoked);
        assert_eq!(
            Some(queued[0].tombstone.subject_ref.clone()),
            link.peer_subject_ref
        );
    }

    #[tokio::test]
    async fn a_user_cannot_revoke_another_users_link() {
        let store = LinkStore::with_peers(vec![peer(PeerStatus::Active)]);
        let outbox = Revocations::default();
        let link = active_link(&store, Uuid::new_v4()).await;

        let outcome =
            revoke(&store, &outbox, link.id, LinkRevoker::User(Uuid::new_v4()))
                .await;

        assert!(outcome.is_err());
        assert_eq!(store.links()[0].status, LinkStatus::Active);
        assert!(outbox.queued.lock().unwrap().is_empty());
    }
}
