use super::accept_link_login::active_peer;
use crate::domain::{
    dtos::federation::{
        IdentityLink, LinkConfirmation, LinkConsent, LinkStatus,
        NewIdentityLink,
    },
    entities::{
        FederationPeerRead, IdentityLinkWrite, LinkConsentRegistration,
        PeerLinkNotifier, UserFetching,
    },
};

use mycelium_base::{
    entities::{CreateResponseKind, FetchResponseKind},
    utils::errors::{use_case_err, MappedErrors},
};
use uuid::Uuid;

/// The user confirmed: record the link and have the consumer record its
/// side (FED-12, FED-46)
///
/// The link is pending until the consumer acknowledges it. If the consumer
/// refuses or cannot be reached, the link is revoked here, so no half-made
/// link ever serves grants. Returns the consent, whose `return_url` is where
/// the browser goes back.
///
#[tracing::instrument(name = "complete_link_on_issuer", skip_all)]
pub async fn complete_link_on_issuer(
    consent_id: Uuid,
    consent_repo: Box<&dyn LinkConsentRegistration>,
    peer_read_repo: Box<&dyn FederationPeerRead>,
    user_fetching_repo: Box<&dyn UserFetching>,
    link_write_repo: Box<&dyn IdentityLinkWrite>,
    notifier: Box<&dyn PeerLinkNotifier>,
) -> Result<LinkConsent, MappedErrors> {
    let FetchResponseKind::Found(consent) =
        consent_repo.consume(consent_id).await?
    else {
        return refused("unknown, used or expired link consent");
    };
    let Some(user_id) = consent.local_user_id else {
        return refused("the link consent has no login");
    };

    let peer = active_peer(consent.peer_id, *peer_read_repo).await?;
    let email = user_email(user_id, *user_fetching_repo).await?;
    let link =
        record_pending(user_id, email, peer.id, *link_write_repo).await?;

    let confirmation = LinkConfirmation {
        request_id: consent.peer_request_id,
        issuer_subject_ref: link.local_subject_ref.clone(),
    };
    let acknowledgement = match notifier.confirm_link(&peer, confirmation).await
    {
        Ok(acknowledgement) => acknowledgement,
        Err(err) => {
            link_write_repo.revoke(link.id, None).await?;
            return Err(err);
        }
    };

    link_write_repo
        .activate(link.id, acknowledgement.consumer_subject_ref)
        .await?;

    Ok(consent)
}

async fn user_email(
    user_id: Uuid,
    user_fetching_repo: &dyn UserFetching,
) -> Result<String, MappedErrors> {
    match user_fetching_repo.get_user_by_id(user_id).await? {
        FetchResponseKind::Found(user) => Ok(user.email.email()),
        FetchResponseKind::NotFound(_) => refused("the linking user is gone"),
    }
}

async fn record_pending(
    local_user_id: Uuid,
    local_email: String,
    peer_id: Uuid,
    link_write_repo: &dyn IdentityLinkWrite,
) -> Result<IdentityLink, MappedErrors> {
    match link_write_repo
        .record(NewIdentityLink {
            local_user_id,
            local_email,
            peer_id,
            peer_subject_ref: None,
            status: LinkStatus::Pending,
        })
        .await?
    {
        CreateResponseKind::Created(link) => Ok(link),
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
        domain::dtos::federation::{NewLinkConsent, PeerStatus},
        use_cases::federation::link_test_doubles::{
            peer, LinkStore, PeerNotifier, UsersWithEmail,
        },
    };

    use chrono::{Duration, Local};

    async fn logged_in_consent(store: &LinkStore) -> Uuid {
        let CreateResponseKind::Created(consent) = store
            .create(NewLinkConsent {
                peer_id: store.peers[0].id,
                peer_request_id: Uuid::new_v4(),
                peer_email: "user@example.com".to_string(),
                return_url: "https://myc-02.example.com/done".to_string(),
                expires_at: Local::now() + Duration::minutes(10),
            })
            .await
            .unwrap()
        else {
            panic!("the consent must be created");
        };

        store.attach_user(consent.id, Uuid::new_v4()).await.unwrap();
        consent.id
    }

    async fn complete(
        store: &LinkStore,
        notifier: &PeerNotifier,
        consent_id: Uuid,
    ) -> Result<LinkConsent, MappedErrors> {
        complete_link_on_issuer(
            consent_id,
            Box::new(store),
            Box::new(store),
            Box::new(&UsersWithEmail("user@example.com")),
            Box::new(store),
            Box::new(notifier),
        )
        .await
    }

    #[tokio::test]
    async fn an_acknowledged_link_is_active_on_the_issuer() {
        let store = LinkStore::with_peers(vec![peer(PeerStatus::Active)]);
        let notifier = PeerNotifier::default();
        let consent_id = logged_in_consent(&store).await;

        let consent = complete(&store, &notifier, consent_id).await.unwrap();

        let links = store.links();
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].status, LinkStatus::Active);
        assert!(links[0].peer_subject_ref.is_some());
        assert_eq!(
            notifier.confirmed.lock().unwrap()[0],
            LinkConfirmation {
                request_id: consent.peer_request_id,
                issuer_subject_ref: links[0].local_subject_ref.clone(),
            }
        );
        assert!(complete(&store, &notifier, consent_id).await.is_err());
    }

    #[tokio::test]
    async fn a_refused_confirmation_leaves_the_link_revoked() {
        let store = LinkStore::with_peers(vec![peer(PeerStatus::Active)]);
        let notifier = PeerNotifier {
            refuses: true,
            ..PeerNotifier::default()
        };
        let consent_id = logged_in_consent(&store).await;

        assert!(complete(&store, &notifier, consent_id).await.is_err());
        assert_eq!(store.links()[0].status, LinkStatus::Revoked);
    }
}
