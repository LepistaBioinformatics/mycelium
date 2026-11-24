//! In-memory ports for the revocation use case tests

use crate::domain::{
    dtos::federation::{
        FederationPeer, OutboxEntry, PendingRevocation, Tombstone,
    },
    entities::{PeerTombstoneSender, RevocationOutbox, TombstoneRegistration},
};

use async_trait::async_trait;
use chrono::{DateTime, Local};
use mycelium_base::utils::errors::{execution_err, MappedErrors};
use std::{sync::Mutex, time::Duration};
use uuid::Uuid;

/// What happened to an outbox row
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Settled {
    Delivered(Uuid),
    Retried(Uuid, DateTime<Local>),
    GaveUp(Uuid),
}

/// The outbox, the sender and the receiver's tombstones, in memory
#[derive(Default)]
pub(crate) struct Revocations {
    pub(crate) queued: Mutex<Vec<PendingRevocation>>,
    pub(crate) due: Mutex<Vec<OutboxEntry>>,
    pub(crate) settled: Mutex<Vec<Settled>>,
    pub(crate) refuses: bool,
    pub(crate) pushed: Mutex<Vec<(Uuid, Tombstone)>>,
    pub(crate) recorded: Mutex<Vec<(Uuid, Tombstone)>>,
}

#[async_trait]
impl RevocationOutbox for Revocations {
    async fn enqueue(
        &self,
        revocations: Vec<PendingRevocation>,
    ) -> Result<u64, MappedErrors> {
        let count = revocations.len() as u64;
        self.queued.lock().unwrap().extend(revocations);
        Ok(count)
    }

    async fn claim_due(
        &self,
        _: u32,
        _: Duration,
    ) -> Result<Vec<OutboxEntry>, MappedErrors> {
        Ok(std::mem::take(&mut *self.due.lock().unwrap()))
    }

    async fn delivered(&self, id: Uuid) -> Result<(), MappedErrors> {
        self.settled.lock().unwrap().push(Settled::Delivered(id));
        Ok(())
    }

    async fn retry(
        &self,
        id: Uuid,
        at: DateTime<Local>,
        _: String,
    ) -> Result<(), MappedErrors> {
        self.settled.lock().unwrap().push(Settled::Retried(id, at));
        Ok(())
    }

    async fn give_up(&self, id: Uuid, _: String) -> Result<(), MappedErrors> {
        self.settled.lock().unwrap().push(Settled::GaveUp(id));
        Ok(())
    }

    async fn purge_failed(
        &self,
        _: DateTime<Local>,
    ) -> Result<u64, MappedErrors> {
        Ok(0)
    }
}

#[async_trait]
impl PeerTombstoneSender for Revocations {
    async fn push(
        &self,
        peer: &FederationPeer,
        tombstone: Tombstone,
    ) -> Result<(), MappedErrors> {
        if self.refuses {
            return execution_err("connection refused").as_error();
        }

        self.pushed.lock().unwrap().push((peer.id, tombstone));
        Ok(())
    }
}

#[async_trait]
impl TombstoneRegistration for Revocations {
    async fn record(
        &self,
        link_id: Uuid,
        _: Uuid,
        tombstone: &Tombstone,
    ) -> Result<(), MappedErrors> {
        self.recorded
            .lock()
            .unwrap()
            .push((link_id, tombstone.clone()));
        Ok(())
    }

    async fn purge_older_than(
        &self,
        _: DateTime<Local>,
    ) -> Result<u64, MappedErrors> {
        Ok(0)
    }
}
