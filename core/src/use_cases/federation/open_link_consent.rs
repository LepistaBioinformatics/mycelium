use crate::domain::{
    dtos::federation::{
        FederationPeer, LinkConsent, NewLinkConsent, PeerStatus,
    },
    entities::LinkConsentRegistration,
};

use chrono::{Duration, Local};
use mycelium_base::{
    entities::CreateResponseKind,
    utils::errors::{use_case_err, MappedErrors},
};
use uuid::Uuid;

/// What a verified redirect state asks the issuer to link
pub struct ConsentAsked {
    pub request_id: Uuid,
    pub peer_email: String,
    pub return_url: String,
}

/// Accept a verified redirect state from an active peer (FED-46)
///
/// The return URL must sit under the peer's declared `base_url`, and each
/// state is accepted once: a replay finds its consent already recorded.
///
#[tracing::instrument(name = "open_link_consent", skip_all)]
pub async fn open_link_consent(
    peer: &FederationPeer,
    asked: ConsentAsked,
    lifetime: Duration,
    consent_repo: Box<&dyn LinkConsentRegistration>,
) -> Result<LinkConsent, MappedErrors> {
    if peer.status != PeerStatus::Active {
        return refused("the requesting peer is not active");
    }
    if !is_under(&peer.base_url, &asked.return_url) {
        return refused("the return URL is outside the peer's base URL");
    }

    match consent_repo
        .create(NewLinkConsent {
            peer_id: peer.id,
            peer_request_id: asked.request_id,
            peer_email: asked.peer_email,
            return_url: asked.return_url,
            expires_at: Local::now() + lifetime,
        })
        .await?
    {
        CreateResponseKind::Created(consent) => Ok(consent),
        CreateResponseKind::NotCreated(..) => {
            refused("this link request was already used")
        }
    }
}

/// `url` is `base_url` itself or a path below it, never a sibling host such
/// as `https://myc-02.example.com.attacker.example`
fn is_under(base_url: &str, url: &str) -> bool {
    let base = base_url.trim_end_matches('/');

    url == base || url.starts_with(&format!("{base}/"))
}

fn refused<T>(reason: &str) -> Result<T, MappedErrors> {
    use_case_err(reason).with_exp_true().as_error()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::use_cases::federation::link_test_doubles::{peer, LinkStore};

    fn asked(return_url: &str) -> ConsentAsked {
        ConsentAsked {
            request_id: Uuid::new_v4(),
            peer_email: "user@example.com".to_string(),
            return_url: return_url.to_string(),
        }
    }

    async fn open(
        peer: &FederationPeer,
        asked: ConsentAsked,
    ) -> Result<LinkConsent, MappedErrors> {
        let store = LinkStore::default();
        open_link_consent(peer, asked, Duration::minutes(10), Box::new(&store))
            .await
    }

    #[tokio::test]
    async fn a_return_url_under_the_peer_is_accepted() {
        let active = peer(PeerStatus::Active);
        let url = "https://myc-02.example.com/federation/link/done";

        let consent = open(&active, asked(url)).await.unwrap();

        assert_eq!(consent.return_url, url);
        assert_eq!(consent.peer_id, active.id);
        assert!(consent.local_user_id.is_none());
    }

    #[tokio::test]
    async fn a_return_url_elsewhere_is_refused() {
        let active = peer(PeerStatus::Active);

        for url in [
            "https://myc-02.example.com.attacker.example/done",
            "https://attacker.example/federation/link/done",
            "http://myc-02.example.com/federation/link/done",
        ] {
            assert!(open(&active, asked(url)).await.is_err(), "{url}");
        }
    }

    #[tokio::test]
    async fn a_quarantined_peer_is_refused() {
        let held = peer(PeerStatus::Quarantined);

        assert!(open(&held, asked("https://myc-02.example.com/done"))
            .await
            .is_err());
    }
}
