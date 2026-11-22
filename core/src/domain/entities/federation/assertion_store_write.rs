use crate::domain::dtos::federation::StoredAssertion;

use async_trait::async_trait;
use mycelium_base::utils::errors::MappedErrors;
use shaku::Interface;
use uuid::Uuid;

/// Writes to the assertion store (D-6, FED-15)
#[async_trait]
pub trait AssertionStoreWrite: Interface + Send + Sync {
    /// Keep the assertion, replacing the link's previous one, unless the
    /// link's tombstone supersedes it (FED-23); false when refused
    async fn store(
        &self,
        assertion: StoredAssertion,
    ) -> Result<bool, MappedErrors>;

    /// Drop the link's assertions, when the peer no longer vouches for them
    async fn forget(&self, link_id: Uuid) -> Result<(), MappedErrors>;

    /// Drop every assertion from the peer (FED-25); returns how many
    async fn forget_peer(&self, peer_id: Uuid) -> Result<u64, MappedErrors>;

    /// Drop every stored assertion (FED-24, at boot); returns how many
    async fn forget_all(&self) -> Result<u64, MappedErrors>;

    /// Drop every expired assertion; returns how many
    async fn purge_expired(&self) -> Result<u64, MappedErrors>;
}
