//! In-memory ports for the resolution use cases

use crate::domain::{
    dtos::{
        federation::{
            FederationPeer, NamespaceClaim, NewFederationPeer, NewPeerKey,
            PeerGrantAnswer, PeerStatus, StoredAssertion, SubjectRef,
        },
        resource_audit_log::NewResourceAuditLogEvent,
        written_by::WrittenBy,
    },
    entities::{
        AssertionStoreRead, AssertionStoreWrite, FederationPeerWrite,
        LocalNamespaceProbe, NamespaceClaimRegistration, PeerGrantResolver,
        ResourceAuditLogRegistration,
    },
};

use async_trait::async_trait;
use mycelium_base::{
    entities::{
        CreateResponseKind, DeletionResponseKind, UpdatingResponseKind,
    },
    utils::errors::{execution_err, MappedErrors},
};
use std::{collections::HashMap, sync::Mutex, time::Duration};
use uuid::Uuid;

/// What a scripted peer does when asked
#[derive(Clone)]
pub(crate) enum Script {
    Answer(PeerGrantAnswer),
    Fail,
    Hang,
}

/// Every port a resolution writes or calls, in memory
#[derive(Default)]
pub(crate) struct Mesh {
    pub(crate) scripts: HashMap<Uuid, Script>,
    pub(crate) asked: Mutex<Vec<Uuid>>,
    pub(crate) stored: Mutex<Vec<StoredAssertion>>,
    pub(crate) local_ids: Vec<Uuid>,
    pub(crate) claims: Mutex<HashMap<Uuid, Uuid>>,
    pub(crate) failures: Mutex<Vec<Uuid>>,
    pub(crate) audited: Mutex<Vec<NewResourceAuditLogEvent>>,
}

#[async_trait]
impl PeerGrantResolver for Mesh {
    async fn resolve(
        &self,
        peer: &FederationPeer,
        _subject_ref: SubjectRef,
    ) -> Result<PeerGrantAnswer, MappedErrors> {
        self.asked.lock().unwrap().push(peer.id);

        match self.scripts.get(&peer.id).cloned() {
            Some(Script::Answer(answer)) => Ok(answer),
            Some(Script::Hang) => {
                tokio::time::sleep(Duration::from_secs(60)).await;
                Ok(PeerGrantAnswer::Unknown)
            }
            _ => execution_err("connection refused").as_error(),
        }
    }
}

#[async_trait]
impl AssertionStoreRead for Mesh {
    async fn list_valid_for_links(
        &self,
        link_ids: &[Uuid],
    ) -> Result<Vec<StoredAssertion>, MappedErrors> {
        let stored = self.stored.lock().unwrap();
        Ok(stored
            .iter()
            .filter(|assertion| link_ids.contains(&assertion.link_id))
            .cloned()
            .collect())
    }

    async fn list_valid(
        &self,
        _peer_id: Option<Uuid>,
    ) -> Result<Vec<StoredAssertion>, MappedErrors> {
        Ok(self.stored.lock().unwrap().clone())
    }
}

#[async_trait]
impl AssertionStoreWrite for Mesh {
    async fn store(
        &self,
        assertion: StoredAssertion,
    ) -> Result<(), MappedErrors> {
        self.stored.lock().unwrap().push(assertion);
        Ok(())
    }

    async fn forget(&self, link_id: Uuid) -> Result<(), MappedErrors> {
        self.stored
            .lock()
            .unwrap()
            .retain(|assertion| assertion.link_id != link_id);
        Ok(())
    }

    async fn purge_expired(&self) -> Result<u64, MappedErrors> {
        Ok(0)
    }
}

#[async_trait]
impl LocalNamespaceProbe for Mesh {
    async fn existing(&self, ids: &[Uuid]) -> Result<Vec<Uuid>, MappedErrors> {
        Ok(ids
            .iter()
            .filter(|id| self.local_ids.contains(id))
            .copied()
            .collect())
    }
}

#[async_trait]
impl NamespaceClaimRegistration for Mesh {
    async fn claim(&self, claim: NamespaceClaim) -> Result<Uuid, MappedErrors> {
        let mut claims = self.claims.lock().unwrap();
        Ok(*claims.entry(claim.foreign_id).or_insert(claim.peer_id))
    }
}

#[async_trait]
impl ResourceAuditLogRegistration for Mesh {
    async fn create(
        &self,
        event: NewResourceAuditLogEvent,
    ) -> Result<(), MappedErrors> {
        self.audited.lock().unwrap().push(event);
        Ok(())
    }
}

#[async_trait]
impl FederationPeerWrite for Mesh {
    async fn declare(
        &self,
        _peer: NewFederationPeer,
        _created_by: Option<WrittenBy>,
    ) -> Result<CreateResponseKind<FederationPeer>, MappedErrors> {
        unimplemented!()
    }

    async fn update_status(
        &self,
        _peer_id: Uuid,
        _status: PeerStatus,
    ) -> Result<UpdatingResponseKind<FederationPeer>, MappedErrors> {
        unimplemented!()
    }

    async fn settle_status(
        &self,
        peer_id: Uuid,
        status: PeerStatus,
    ) -> Result<UpdatingResponseKind<FederationPeer>, MappedErrors> {
        let mut peer = super::link_test_doubles::peer(status);
        peer.id = peer_id;
        Ok(UpdatingResponseKind::Updated(peer))
    }

    async fn record_keys(
        &self,
        _peer_id: Uuid,
        _keys: Vec<NewPeerKey>,
    ) -> Result<(), MappedErrors> {
        Ok(())
    }

    async fn set_negotiated_kinds(
        &self,
        _peer_id: Uuid,
        _kinds: Vec<String>,
    ) -> Result<(), MappedErrors> {
        Ok(())
    }

    async fn record_success(&self, _peer_id: Uuid) -> Result<(), MappedErrors> {
        Ok(())
    }

    async fn record_failure(&self, peer_id: Uuid) -> Result<u32, MappedErrors> {
        let mut failures = self.failures.lock().unwrap();
        failures.push(peer_id);
        Ok(failures.len() as u32)
    }

    async fn remove(
        &self,
        _peer_id: Uuid,
    ) -> Result<DeletionResponseKind<Uuid>, MappedErrors> {
        Ok(DeletionResponseKind::Deleted)
    }
}
