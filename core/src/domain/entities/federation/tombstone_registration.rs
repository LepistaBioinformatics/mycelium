use crate::domain::dtos::federation::Tombstone;

use async_trait::async_trait;
use chrono::{DateTime, Local};
use mycelium_base::utils::errors::MappedErrors;
use shaku::Interface;
use uuid::Uuid;

/// Received tombstones, durable and shared by every pod (FED-23)
#[async_trait]
pub trait TombstoneRegistration: Interface + Send + Sync {
    /// Keep the latest tombstone of the link and drop its stored assertion,
    /// atomically; an older or repeated tombstone changes nothing
    async fn record(
        &self,
        link_id: Uuid,
        peer_id: Uuid,
        tombstone: &Tombstone,
    ) -> Result<(), MappedErrors>;

    /// Drop tombstones received before `before`
    async fn purge_older_than(
        &self,
        before: DateTime<Local>,
    ) -> Result<u64, MappedErrors>;
}
