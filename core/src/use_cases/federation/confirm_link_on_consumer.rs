use crate::domain::{
    dtos::federation::{
        FederationPeer, LinkAcknowledgement, LinkConfirmation, LinkStatus,
        NewIdentityLink,
    },
    entities::{IdentityLinkWrite, LinkRequestRegistration, UserFetching},
};

use mycelium_base::{
    entities::{CreateResponseKind, FetchResponseKind},
    utils::errors::{use_case_err, MappedErrors},
};

/// The issuer says the user confirmed: record the link on this side
/// (FED-12, FED-42)
///
/// `peer` is the verified caller. The request must have been made towards
/// it, redirected and not yet consumed; consuming it makes the confirmation
/// single-use.
///
#[tracing::instrument(name = "confirm_link_on_consumer", skip_all)]
pub async fn confirm_link_on_consumer(
    peer: &FederationPeer,
    confirmation: LinkConfirmation,
    request_repo: Box<&dyn LinkRequestRegistration>,
    user_fetching_repo: Box<&dyn UserFetching>,
    link_write_repo: Box<&dyn IdentityLinkWrite>,
) -> Result<LinkAcknowledgement, MappedErrors> {
    let FetchResponseKind::Found(request) = request_repo
        .consume(confirmation.request_id, peer.id)
        .await?
    else {
        return refused("unknown, used or expired link request");
    };

    let FetchResponseKind::Found(user) = user_fetching_repo
        .get_user_by_id(request.local_user_id)
        .await?
    else {
        return refused("the user of this link request is gone");
    };

    match link_write_repo
        .record(NewIdentityLink {
            local_user_id: request.local_user_id,
            local_email: user.email.email(),
            peer_id: peer.id,
            peer_subject_ref: Some(confirmation.issuer_subject_ref),
            status: LinkStatus::Active,
        })
        .await?
    {
        CreateResponseKind::Created(link) => Ok(LinkAcknowledgement {
            consumer_subject_ref: link.local_subject_ref,
        }),
        CreateResponseKind::NotCreated(..) => {
            refused("the account is already linked to this peer")
        }
    }
}

fn refused<T>(reason: &str) -> Result<T, MappedErrors> {
    use_case_err(reason).with_exp_true().as_error()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::{
            dtos::federation::{
                LinkRequestSecret, NewLinkRequest, PeerStatus, SubjectRef,
            },
            entities::LinkRequestRegistration,
        },
        use_cases::federation::link_test_doubles::{
            peer, LinkStore, UsersWithEmail,
        },
    };

    use chrono::{Duration, Local};
    use uuid::Uuid;

    async fn redirected_request(store: &LinkStore) -> Uuid {
        let secret = LinkRequestSecret::mint();
        let request = store
            .create(NewLinkRequest {
                local_user_id: Uuid::new_v4(),
                peer_id: store.peers[0].id,
                secret_hash: secret.hash(),
                expires_at: Local::now() + Duration::minutes(10),
            })
            .await
            .unwrap();
        store.redirect(request.id, secret.hash()).await.unwrap();

        request.id
    }

    async fn confirm(
        store: &LinkStore,
        caller: &FederationPeer,
        request_id: Uuid,
    ) -> Result<LinkAcknowledgement, MappedErrors> {
        confirm_link_on_consumer(
            caller,
            LinkConfirmation {
                request_id,
                issuer_subject_ref: SubjectRef::mint(),
            },
            Box::new(store),
            Box::new(&UsersWithEmail("user@example.com")),
            Box::new(store),
        )
        .await
    }

    #[tokio::test]
    async fn the_requested_peer_confirms_once() {
        let store = LinkStore::with_peers(vec![peer(PeerStatus::Active)]);
        let caller = store.peers[0].clone();
        let request_id = redirected_request(&store).await;

        let acknowledgement =
            confirm(&store, &caller, request_id).await.unwrap();

        let links = store.links();
        assert_eq!(links[0].status, LinkStatus::Active);
        assert_eq!(
            links[0].local_subject_ref,
            acknowledgement.consumer_subject_ref
        );
        assert!(confirm(&store, &caller, request_id).await.is_err());
    }

    #[tokio::test]
    async fn another_peer_cannot_confirm_the_request() {
        let store = LinkStore::with_peers(vec![peer(PeerStatus::Active)]);
        let request_id = redirected_request(&store).await;

        let stranger = peer(PeerStatus::Active);
        assert!(confirm(&store, &stranger, request_id).await.is_err());
        assert!(store.links().is_empty());
    }
}
