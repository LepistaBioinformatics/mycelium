use crate::domain::dtos::federation::{OutboxEntry, PendingRevocation};

use async_trait::async_trait;
use chrono::{DateTime, Local};
use mycelium_base::utils::errors::MappedErrors;
use shaku::Interface;
use std::time::Duration;
use uuid::Uuid;

/// Tombstones to push, retried until delivered (FED-24)
///
/// A claimed row is leased: no other pod claims it before the lease ends,
/// and a pod that dies mid-delivery leaves it to the next claim.
///
#[async_trait]
pub trait RevocationOutbox: Interface + Send + Sync {
    /// Queue the revocations, due at once; returns how many
    async fn enqueue(
        &self,
        revocations: Vec<PendingRevocation>,
    ) -> Result<u64, MappedErrors>;

    /// Lease up to `limit` pending rows whose next attempt is due
    async fn claim_due(
        &self,
        limit: u32,
        lease: Duration,
    ) -> Result<Vec<OutboxEntry>, MappedErrors>;

    async fn delivered(&self, id: Uuid) -> Result<(), MappedErrors>;

    /// Count a failed attempt and schedule the next at `next_attempt_at`
    async fn retry(
        &self,
        id: Uuid,
        next_attempt_at: DateTime<Local>,
        error: String,
    ) -> Result<(), MappedErrors>;

    /// Stop retrying the row; it stays for operators until purged
    async fn give_up(
        &self,
        id: Uuid,
        error: String,
    ) -> Result<(), MappedErrors>;

    /// Drop rows given up before `before`
    async fn purge_failed(
        &self,
        before: DateTime<Local>,
    ) -> Result<u64, MappedErrors>;
}
