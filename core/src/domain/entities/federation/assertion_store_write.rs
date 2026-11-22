use crate::domain::dtos::federation::StoredAssertion;

use async_trait::async_trait;
use mycelium_base::utils::errors::MappedErrors;
use shaku::Interface;

/// Writes to the assertion store (D-6, FED-15)
#[async_trait]
pub trait AssertionStoreWrite: Interface + Send + Sync {
    /// Keep the assertion, replacing the link's previous one
    async fn store(
        &self,
        assertion: StoredAssertion,
    ) -> Result<(), MappedErrors>;

    /// Drop every expired assertion; returns how many
    async fn purge_expired(&self) -> Result<u64, MappedErrors>;
}
