use crate::domain::{
    dtos::federation::{FederationPeer, LinkConsent, PeerStatus},
    entities::{FederationPeerRead, LinkConsentRegistration},
};

use mycelium_base::{
    entities::FetchResponseKind,
    utils::errors::{use_case_err, MappedErrors},
};
use uuid::Uuid;

/// The user who logged in on the issuer during a link
pub struct LinkingUser {
    pub id: Uuid,
    pub email: String,
}

/// Bind the user who just logged in to the consent (FED-44, FED-45)
///
/// The emails must match, case aside, unless the peer's policy allows
/// different ones (D-7). A consent takes one login only.
///
#[tracing::instrument(name = "accept_link_login", skip_all)]
pub async fn accept_link_login(
    consent_id: Uuid,
    user: LinkingUser,
    consent_repo: Box<&dyn LinkConsentRegistration>,
    peer_read_repo: Box<&dyn FederationPeerRead>,
) -> Result<(LinkConsent, FederationPeer), MappedErrors> {
    let FetchResponseKind::Found(consent) =
        consent_repo.get_open(consent_id).await?
    else {
        return refused("unknown, used or expired link consent");
    };

    let peer = active_peer(consent.peer_id, *peer_read_repo).await?;
    let same_email = consent.peer_email.eq_ignore_ascii_case(&user.email);
    if !same_email && !peer.policy.allow_different_emails {
        return refused("the accounts to link have different emails");
    }

    match consent_repo.attach_user(consent_id, user.id).await? {
        FetchResponseKind::Found(consent) => Ok((consent, peer)),
        FetchResponseKind::NotFound(_) => {
            refused("this link consent already has a login")
        }
    }
}

pub(super) async fn active_peer(
    peer_id: Uuid,
    peer_read_repo: &dyn FederationPeerRead,
) -> Result<FederationPeer, MappedErrors> {
    match peer_read_repo.get(peer_id).await? {
        FetchResponseKind::Found(peer) if peer.status == PeerStatus::Active => {
            Ok(peer)
        }
        _ => refused("the peer of this link is not active"),
    }
}

fn refused<T>(reason: &str) -> Result<T, MappedErrors> {
    use_case_err(reason).with_exp_true().as_error()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::dtos::federation::NewLinkConsent,
        use_cases::federation::link_test_doubles::{peer, LinkStore},
    };

    use chrono::{Duration, Local};

    async fn store_with_consent(
        allow_different_emails: bool,
    ) -> (LinkStore, Uuid) {
        let mut active = peer(PeerStatus::Active);
        active.policy.allow_different_emails = allow_different_emails;
        let store = LinkStore::with_peers(vec![active.clone()]);
        let consent = store
            .create(NewLinkConsent {
                peer_id: active.id,
                peer_request_id: Uuid::new_v4(),
                peer_email: "User@Example.com".to_string(),
                return_url: "https://myc-02.example.com/done".to_string(),
                expires_at: Local::now() + Duration::minutes(10),
            })
            .await
            .unwrap();

        let mycelium_base::entities::CreateResponseKind::Created(consent) =
            consent
        else {
            panic!("the consent must be created");
        };

        (store, consent.id)
    }

    async fn login(
        store: &LinkStore,
        consent_id: Uuid,
        email: &str,
    ) -> Result<(LinkConsent, FederationPeer), MappedErrors> {
        accept_link_login(
            consent_id,
            LinkingUser {
                id: Uuid::new_v4(),
                email: email.to_string(),
            },
            Box::new(store),
            Box::new(store),
        )
        .await
    }

    #[tokio::test]
    async fn the_same_email_in_another_case_is_accepted_once() {
        let (store, consent_id) = store_with_consent(false).await;

        let (consent, _) =
            login(&store, consent_id, "user@example.com").await.unwrap();
        assert!(consent.local_user_id.is_some());

        assert!(login(&store, consent_id, "user@example.com").await.is_err());
    }

    #[tokio::test]
    async fn a_different_email_needs_the_peer_policy() {
        let (store, consent_id) = store_with_consent(false).await;
        assert!(login(&store, consent_id, "other@example.com")
            .await
            .is_err());

        let (store, consent_id) = store_with_consent(true).await;
        assert!(login(&store, consent_id, "other@example.com").await.is_ok());
    }
}
