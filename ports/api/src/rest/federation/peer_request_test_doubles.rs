//! In-process doubles of the federation ports, so the peer-authenticated
//! endpoints are tested over HTTP without a database or a real peer.

use crate::federation::{FederationSigning, PreparedFederation};

use actix_web::web;
use async_trait::async_trait;
use chrono::{DateTime, Local, Utc};
use myc_core::{
    domain::{
        dtos::{
            federation::{
                FederationPeer, FederationPolicy, InstanceSelfDescription,
                NewFederationPeer, NewPeerKey, PeerKey, PeerStatus,
            },
            instance_settings::InstanceSetting,
            profile_signature::SigningKeyVersion,
            written_by::WrittenBy,
        },
        entities::{
            FederationPeerRead, FederationPeerWrite, InstanceSettingsFetching,
            PeerClient, RequestNonceRegistration,
        },
    },
    models::FederationConfig,
};
use myc_http_tools::{
    federation::{sign_peer_request, PeerRequest},
    models::{KeyDerivationPurpose, VersionedEd25519Signer},
};
use mycelium_base::{
    entities::{
        CreateResponseKind, DeletionResponseKind, FetchManyResponseKind,
        FetchResponseKind, UpdatingResponseKind,
    },
    utils::errors::{execution_err, MappedErrors},
};
use std::{
    collections::HashSet,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
};
use uuid::Uuid;

pub(crate) fn signer(secret: &[u8]) -> VersionedEd25519Signer {
    VersionedEd25519Signer::derive_from_secret(
        SigningKeyVersion::try_from(1).unwrap(),
        secret,
        KeyDerivationPurpose::Federation,
    )
    .unwrap()
}

/// One declared peer, its stored key, and what was done to it
pub(crate) struct PeerBook {
    pub(crate) peer: FederationPeer,
    pub(crate) key: PeerKey,
    pub(crate) key_reads: AtomicUsize,
    pub(crate) statuses: Mutex<Vec<PeerStatus>>,
}

impl PeerBook {
    pub(crate) fn key_reads(&self) -> usize {
        self.key_reads.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl FederationPeerRead for PeerBook {
    async fn get(
        &self,
        _: Uuid,
    ) -> Result<FetchResponseKind<FederationPeer, Uuid>, MappedErrors> {
        unreachable!()
    }

    async fn get_by_instance_id(
        &self,
        instance_id: Uuid,
    ) -> Result<FetchResponseKind<FederationPeer, Uuid>, MappedErrors> {
        match self.peer.instance_id == instance_id {
            true => Ok(FetchResponseKind::Found(self.peer.clone())),
            false => Ok(FetchResponseKind::NotFound(Some(instance_id))),
        }
    }

    async fn list(
        &self,
    ) -> Result<FetchManyResponseKind<FederationPeer>, MappedErrors> {
        unreachable!()
    }

    async fn list_keys(&self, _: Uuid) -> Result<Vec<PeerKey>, MappedErrors> {
        self.key_reads.fetch_add(1, Ordering::SeqCst);
        Ok(vec![self.key.clone()])
    }
}

#[async_trait]
impl FederationPeerWrite for PeerBook {
    async fn declare(
        &self,
        _: NewFederationPeer,
        _: Option<WrittenBy>,
    ) -> Result<CreateResponseKind<FederationPeer>, MappedErrors> {
        unreachable!()
    }

    async fn update_status(
        &self,
        _: Uuid,
        status: PeerStatus,
    ) -> Result<UpdatingResponseKind<FederationPeer>, MappedErrors> {
        self.statuses.lock().unwrap().push(status);
        Ok(UpdatingResponseKind::Updated(self.peer.clone()))
    }

    async fn record_keys(
        &self,
        _: Uuid,
        _: Vec<NewPeerKey>,
    ) -> Result<(), MappedErrors> {
        Ok(())
    }

    async fn set_negotiated_kinds(
        &self,
        _: Uuid,
        _: Vec<String>,
    ) -> Result<(), MappedErrors> {
        Ok(())
    }

    async fn record_success(&self, _: Uuid) -> Result<(), MappedErrors> {
        Ok(())
    }

    async fn record_failure(&self, _: Uuid) -> Result<u32, MappedErrors> {
        Ok(1)
    }

    async fn remove(
        &self,
        _: Uuid,
    ) -> Result<DeletionResponseKind<Uuid>, MappedErrors> {
        unreachable!()
    }
}

#[derive(Default)]
pub(crate) struct SeenNonces(Mutex<HashSet<String>>);

#[async_trait]
impl RequestNonceRegistration for SeenNonces {
    async fn claim(
        &self,
        jti: String,
        _: Uuid,
        _: DateTime<Local>,
    ) -> Result<bool, MappedErrors> {
        Ok(self.0.lock().unwrap().insert(jti))
    }

    async fn purge_expired(&self) -> Result<u64, MappedErrors> {
        Ok(0)
    }
}

/// The runtime federation switch, on or off
pub(crate) struct Switch(pub(crate) bool);

#[async_trait]
impl InstanceSettingsFetching for Switch {
    async fn get(
        &self,
        key: String,
    ) -> Result<FetchResponseKind<InstanceSetting, ()>, MappedErrors> {
        Ok(FetchResponseKind::Found(InstanceSetting {
            key,
            value: serde_json::json!(self.0),
            created_by: None,
            updated_by: None,
            created: Local::now(),
            updated: None,
        }))
    }
}

/// A peer that answers with `description`, or is unreachable when `None`
pub(crate) struct RemotePeer {
    pub(crate) description: Option<InstanceSelfDescription>,
    pub(crate) calls: AtomicUsize,
}

impl RemotePeer {
    pub(crate) fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl PeerClient for RemotePeer {
    async fn fetch_self_description(
        &self,
        _: &str,
    ) -> Result<InstanceSelfDescription, MappedErrors> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.description
            .clone()
            .ok_or_else(|| execution_err("peer unreachable"))
    }

    async fn post_signed(
        &self,
        _: &str,
        _: &str,
        _: Vec<u8>,
        _: &str,
    ) -> Result<Vec<u8>, MappedErrors> {
        unreachable!()
    }
}

/// This instance, one declared peer, and the doubles both sides talk through
pub(crate) struct FederationWorld {
    pub(crate) local: PreparedFederation,
    pub(crate) peer_signer: VersionedEd25519Signer,
    pub(crate) peers: Arc<PeerBook>,
    pub(crate) nonces: Arc<SeenNonces>,
    pub(crate) remote: Arc<RemotePeer>,
    pub(crate) enabled: bool,
}

pub(crate) fn federation_world(status: PeerStatus) -> FederationWorld {
    let local_signer = signer(b"local-test-secret");
    let peer_signer = signer(b"peer-test-secret");
    let peer = declared_peer(status);
    let key = PeerKey {
        peer_id: peer.id,
        kid: "1".to_string(),
        alg: "EdDSA".to_string(),
        public_key: peer_signer.public_jwk().x(),
        fetched_at: Local::now(),
        expires_at: None,
    };
    let remote = RemotePeer {
        description: Some(remote_description(peer.instance_id)),
        calls: AtomicUsize::new(0),
    };

    FederationWorld {
        local: prepared(local_signer),
        peer_signer,
        peers: Arc::new(PeerBook {
            peer,
            key,
            key_reads: AtomicUsize::new(0),
            statuses: Mutex::new(vec![]),
        }),
        nonces: Arc::new(SeenNonces::default()),
        remote: Arc::new(remote),
        enabled: true,
    }
}

fn prepared(signer: VersionedEd25519Signer) -> PreparedFederation {
    PreparedFederation {
        config: serde_json::from_value::<FederationConfig>(
            serde_json::json!({"baseUrl": "https://myc-01.example.com"}),
        )
        .unwrap(),
        instance_id: Uuid::new_v4(),
        signing: FederationSigning {
            public_keys: vec![signer.public_jwk().clone()],
            primary: Arc::new(signer),
        },
    }
}

fn declared_peer(status: PeerStatus) -> FederationPeer {
    FederationPeer {
        id: Uuid::new_v4(),
        instance_id: Uuid::new_v4(),
        base_url: "https://myc-02.example.com".to_string(),
        display_name: None,
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

fn remote_description(instance_id: Uuid) -> InstanceSelfDescription {
    InstanceSelfDescription {
        instance_id,
        display_name: None,
        base_url: "https://myc-02.example.com".to_string(),
        version: "9.0.1".to_string(),
        assertion_kinds: vec![],
        keys: vec![],
        role_catalog_url: None,
    }
}

impl FederationWorld {
    /// A request signed by the declared peer's key, claiming `issuer`
    pub(crate) fn sign(
        &self,
        issuer: Uuid,
        path: &str,
        jti: &str,
        body: &[u8],
    ) -> String {
        sign_peer_request(
            &self.peer_signer,
            &PeerRequest {
                iss: issuer,
                aud: self.local.instance_id,
                method: "POST",
                path,
                body,
                jti,
                issued_at: Utc::now().timestamp() as u64,
                lifetime_secs: 60,
            },
        )
        .unwrap()
    }

    /// Register every federation port of this world on an actix app
    pub(crate) fn configure(&self, config: &mut web::ServiceConfig) {
        let read: Arc<dyn FederationPeerRead> = self.peers.clone();
        let write: Arc<dyn FederationPeerWrite> = self.peers.clone();
        let nonces: Arc<dyn RequestNonceRegistration> = self.nonces.clone();
        let settings: Arc<dyn InstanceSettingsFetching> =
            Arc::new(Switch(self.enabled));
        let remote: Arc<dyn PeerClient> = self.remote.clone();

        config
            .app_data(web::Data::new(Some(self.local.clone())))
            .app_data(web::Data::from(read))
            .app_data(web::Data::from(write))
            .app_data(web::Data::from(nonces))
            .app_data(web::Data::from(settings))
            .app_data(web::Data::from(remote));
    }
}
