//! In-memory ports for the identity link use case tests

use crate::domain::{
    dtos::{
        federation::{
            FederationPeer, FederationPolicy, IdentityLink, LinkConsent,
            LinkRequest, LinkStatus, NewIdentityLink, NewLinkConsent,
            NewLinkRequest, PeerKey, PeerStatus, SubjectRef,
        },
        written_by::WrittenBy,
    },
    entities::{
        FederationPeerRead, IdentityLinkRead, IdentityLinkWrite,
        LinkConsentRegistration, LinkRequestRegistration,
    },
};

use async_trait::async_trait;
use chrono::Local;
use mycelium_base::{
    entities::{
        CreateResponseKind, FetchManyResponseKind, FetchResponseKind,
        UpdatingResponseKind,
    },
    utils::errors::MappedErrors,
};
use std::sync::Mutex;
use uuid::Uuid;

pub(crate) fn peer(status: PeerStatus) -> FederationPeer {
    FederationPeer {
        id: Uuid::new_v4(),
        instance_id: Uuid::new_v4(),
        base_url: "https://myc-02.example.com".to_string(),
        display_name: Some("Instance 02".to_string()),
        status,
        consume_kinds: vec![],
        serve_kinds: vec![],
        negotiated_kinds: vec![],
        policy: FederationPolicy::default(),
        last_contact_at: None,
        failure_count: 0,
        created: Local::now(),
        created_by: None,
        updated: None,
        updated_by: None,
    }
}

/// Peers, links, link requests and consents held in memory
#[derive(Default)]
pub(crate) struct LinkStore {
    pub(crate) peers: Vec<FederationPeer>,
    pub(crate) links: Mutex<Vec<IdentityLink>>,
    pub(crate) requests: Mutex<Vec<(LinkRequest, NewLinkRequest)>>,
    pub(crate) consents: Mutex<Vec<LinkConsent>>,
}

impl LinkStore {
    pub(crate) fn with_peers(peers: Vec<FederationPeer>) -> Self {
        Self {
            peers,
            ..Self::default()
        }
    }

    pub(crate) fn links(&self) -> Vec<IdentityLink> {
        self.links.lock().unwrap().clone()
    }

    fn change_link(
        &self,
        link_id: Uuid,
        allowed: impl Fn(&IdentityLink) -> bool,
        change: impl Fn(&mut IdentityLink),
    ) -> UpdatingResponseKind<IdentityLink> {
        let mut links = self.links.lock().unwrap();
        let link = links.iter_mut().find(|link| link.id == link_id).unwrap();

        if !allowed(link) {
            return UpdatingResponseKind::NotUpdated(
                link.clone(),
                "refused".to_string(),
            );
        }

        change(link);
        UpdatingResponseKind::Updated(link.clone())
    }
}

fn found<T: serde::Serialize, U>(
    value: Option<T>,
    key: U,
) -> FetchResponseKind<T, U> {
    match value {
        Some(value) => FetchResponseKind::Found(value),
        None => FetchResponseKind::NotFound(Some(key)),
    }
}

#[async_trait]
impl FederationPeerRead for LinkStore {
    async fn get(
        &self,
        peer_id: Uuid,
    ) -> Result<FetchResponseKind<FederationPeer, Uuid>, MappedErrors> {
        let peer = self.peers.iter().find(|peer| peer.id == peer_id);

        Ok(found(peer.cloned(), peer_id))
    }

    async fn get_by_instance_id(
        &self,
        instance_id: Uuid,
    ) -> Result<FetchResponseKind<FederationPeer, Uuid>, MappedErrors> {
        let peer = self.peers.iter().find(|p| p.instance_id == instance_id);

        Ok(found(peer.cloned(), instance_id))
    }

    async fn list(
        &self,
    ) -> Result<FetchManyResponseKind<FederationPeer>, MappedErrors> {
        Ok(FetchManyResponseKind::Found(self.peers.clone()))
    }

    async fn list_keys(&self, _: Uuid) -> Result<Vec<PeerKey>, MappedErrors> {
        Ok(vec![])
    }
}

#[async_trait]
impl IdentityLinkRead for LinkStore {
    async fn get(
        &self,
        link_id: Uuid,
    ) -> Result<FetchResponseKind<IdentityLink, Uuid>, MappedErrors> {
        let link = self.links().into_iter().find(|link| link.id == link_id);

        Ok(found(link, link_id))
    }

    async fn find_live(
        &self,
        local_user_id: Uuid,
        peer_id: Uuid,
    ) -> Result<FetchResponseKind<IdentityLink, Uuid>, MappedErrors> {
        let link = self.links().into_iter().find(|link| {
            link.local_user_id == local_user_id
                && link.peer_id == peer_id
                && link.status != LinkStatus::Revoked
        });

        Ok(found(link, peer_id))
    }

    async fn find_by_local_ref(
        &self,
        peer_id: Uuid,
        local_subject_ref: SubjectRef,
    ) -> Result<FetchResponseKind<IdentityLink, SubjectRef>, MappedErrors> {
        let link = self.links().into_iter().find(|link| {
            link.peer_id == peer_id
                && link.local_subject_ref == local_subject_ref
        });

        Ok(found(link, local_subject_ref))
    }

    async fn list(
        &self,
        local_user_id: Option<Uuid>,
    ) -> Result<Vec<IdentityLink>, MappedErrors> {
        Ok(self
            .links()
            .into_iter()
            .filter(|link| {
                local_user_id.is_none_or(|id| link.local_user_id == id)
            })
            .collect())
    }

    async fn list_active_by_emails(
        &self,
        emails: &[String],
    ) -> Result<Vec<IdentityLink>, MappedErrors> {
        Ok(self
            .links()
            .into_iter()
            .filter(|link| link.status == LinkStatus::Active)
            .filter(|link| emails.contains(&link.local_email))
            .collect())
    }
}

#[async_trait]
impl IdentityLinkWrite for LinkStore {
    async fn record(
        &self,
        link: NewIdentityLink,
    ) -> Result<CreateResponseKind<IdentityLink>, MappedErrors> {
        let recorded = IdentityLink {
            id: Uuid::new_v4(),
            local_user_id: link.local_user_id,
            local_email: link.local_email,
            peer_id: link.peer_id,
            established_at: (link.status == LinkStatus::Active)
                .then(Local::now),
            peer_subject_ref: link.peer_subject_ref,
            local_subject_ref: SubjectRef::mint(),
            status: link.status,
            created: Local::now(),
            revoked_at: None,
            revoked_by: None,
        };

        self.links.lock().unwrap().push(recorded.clone());
        Ok(CreateResponseKind::Created(recorded))
    }

    async fn activate(
        &self,
        link_id: Uuid,
        peer_subject_ref: SubjectRef,
    ) -> Result<UpdatingResponseKind<IdentityLink>, MappedErrors> {
        Ok(self.change_link(
            link_id,
            |link| link.status == LinkStatus::Pending,
            |link| {
                link.status = LinkStatus::Active;
                link.peer_subject_ref = Some(peer_subject_ref.clone());
            },
        ))
    }

    async fn revoke(
        &self,
        link_id: Uuid,
        revoked_by: Option<WrittenBy>,
    ) -> Result<UpdatingResponseKind<IdentityLink>, MappedErrors> {
        Ok(self.change_link(
            link_id,
            |link| link.status != LinkStatus::Revoked,
            |link| {
                link.status = LinkStatus::Revoked;
                link.revoked_by = revoked_by.clone();
            },
        ))
    }
}

#[async_trait]
impl LinkRequestRegistration for LinkStore {
    async fn create(
        &self,
        request: NewLinkRequest,
    ) -> Result<LinkRequest, MappedErrors> {
        let created = LinkRequest {
            id: Uuid::new_v4(),
            local_user_id: request.local_user_id,
            peer_id: request.peer_id,
            expires_at: request.expires_at,
            redirected_at: None,
        };

        self.requests
            .lock()
            .unwrap()
            .push((created.clone(), request));
        Ok(created)
    }

    async fn get_open(
        &self,
        request_id: Uuid,
        secret_hash: String,
    ) -> Result<FetchResponseKind<LinkRequest, Uuid>, MappedErrors> {
        let requests = self.requests.lock().unwrap();
        let request = requests.iter().find(|(request, asked)| {
            request.id == request_id
                && asked.secret_hash == secret_hash
                && request.redirected_at.is_none()
        });

        Ok(found(
            request.map(|(request, _)| request.clone()),
            request_id,
        ))
    }

    async fn redirect(
        &self,
        request_id: Uuid,
        secret_hash: String,
    ) -> Result<FetchResponseKind<LinkRequest, Uuid>, MappedErrors> {
        let mut requests = self.requests.lock().unwrap();
        let request = requests.iter_mut().find(|(request, asked)| {
            request.id == request_id
                && asked.secret_hash == secret_hash
                && request.redirected_at.is_none()
        });

        Ok(found(
            request.map(|(request, _)| {
                request.redirected_at = Some(Local::now());
                request.clone()
            }),
            request_id,
        ))
    }

    async fn consume(
        &self,
        request_id: Uuid,
        peer_id: Uuid,
    ) -> Result<FetchResponseKind<LinkRequest, Uuid>, MappedErrors> {
        let mut requests = self.requests.lock().unwrap();
        let index = requests.iter().position(|(request, _)| {
            request.id == request_id
                && request.peer_id == peer_id
                && request.redirected_at.is_some()
        });

        Ok(found(index.map(|i| requests.remove(i).0), request_id))
    }

    async fn purge_expired(&self) -> Result<u64, MappedErrors> {
        Ok(0)
    }
}

#[async_trait]
impl LinkConsentRegistration for LinkStore {
    async fn create(
        &self,
        consent: NewLinkConsent,
    ) -> Result<CreateResponseKind<LinkConsent>, MappedErrors> {
        let created = LinkConsent {
            id: Uuid::new_v4(),
            peer_id: consent.peer_id,
            peer_request_id: consent.peer_request_id,
            peer_email: consent.peer_email,
            return_url: consent.return_url,
            local_user_id: None,
            expires_at: consent.expires_at,
        };

        self.consents.lock().unwrap().push(created.clone());
        Ok(CreateResponseKind::Created(created))
    }

    async fn get_open(
        &self,
        consent_id: Uuid,
    ) -> Result<FetchResponseKind<LinkConsent, Uuid>, MappedErrors> {
        let consents = self.consents.lock().unwrap();
        let consent = consents.iter().find(|consent| consent.id == consent_id);

        Ok(found(consent.cloned(), consent_id))
    }

    async fn attach_user(
        &self,
        consent_id: Uuid,
        local_user_id: Uuid,
    ) -> Result<FetchResponseKind<LinkConsent, Uuid>, MappedErrors> {
        let mut consents = self.consents.lock().unwrap();
        let consent = consents.iter_mut().find(|consent| {
            consent.id == consent_id && consent.local_user_id.is_none()
        });

        Ok(found(
            consent.map(|consent| {
                consent.local_user_id = Some(local_user_id);
                consent.clone()
            }),
            consent_id,
        ))
    }

    async fn consume(
        &self,
        consent_id: Uuid,
    ) -> Result<FetchResponseKind<LinkConsent, Uuid>, MappedErrors> {
        let mut consents = self.consents.lock().unwrap();
        let index = consents.iter().position(|consent| {
            consent.id == consent_id && consent.local_user_id.is_some()
        });

        Ok(found(index.map(|i| consents.remove(i)), consent_id))
    }

    async fn purge_expired(&self) -> Result<u64, MappedErrors> {
        Ok(0)
    }
}

/// Every user exists, with this email
pub(crate) struct UsersWithEmail(pub(crate) &'static str);

#[async_trait]
impl crate::domain::entities::UserFetching for UsersWithEmail {
    async fn get_user_by_email(
        &self,
        _: crate::domain::dtos::email::Email,
    ) -> Result<
        FetchResponseKind<crate::domain::dtos::user::User, String>,
        MappedErrors,
    > {
        unimplemented!()
    }

    async fn get_user_by_id(
        &self,
        id: Uuid,
    ) -> Result<
        FetchResponseKind<crate::domain::dtos::user::User, String>,
        MappedErrors,
    > {
        let email =
            crate::domain::dtos::email::Email::from_string(self.0.to_string())?;
        let mut user =
            crate::domain::dtos::user::User::new_principal_with_provider(
                None,
                email,
                crate::domain::dtos::user::Provider::External(
                    "test".to_string(),
                ),
                None,
                None,
            )?;
        user.id = Some(id);

        Ok(FetchResponseKind::Found(user))
    }

    async fn get_not_redacted_user_by_email(
        &self,
        _: crate::domain::dtos::email::Email,
    ) -> Result<
        FetchResponseKind<crate::domain::dtos::user::User, String>,
        MappedErrors,
    > {
        unimplemented!()
    }
}

/// A peer that acknowledges every confirmation, or refuses them all
#[derive(Default)]
pub(crate) struct PeerNotifier {
    pub(crate) refuses: bool,
    pub(crate) confirmed:
        Mutex<Vec<crate::domain::dtos::federation::LinkConfirmation>>,
}

#[async_trait]
impl crate::domain::entities::PeerLinkNotifier for PeerNotifier {
    async fn confirm_link(
        &self,
        _: &FederationPeer,
        confirmation: crate::domain::dtos::federation::LinkConfirmation,
    ) -> Result<
        crate::domain::dtos::federation::LinkAcknowledgement,
        MappedErrors,
    > {
        if self.refuses {
            return mycelium_base::utils::errors::execution_err("refused")
                .as_error();
        }

        self.confirmed.lock().unwrap().push(confirmation);
        Ok(crate::domain::dtos::federation::LinkAcknowledgement {
            consumer_subject_ref: SubjectRef::mint(),
        })
    }
}
