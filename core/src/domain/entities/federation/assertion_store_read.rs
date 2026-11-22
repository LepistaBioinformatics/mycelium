use crate::domain::dtos::federation::StoredAssertion;

use async_trait::async_trait;
use mycelium_base::utils::errors::MappedErrors;
use shaku::Interface;
use uuid::Uuid;

/// Reads of the assertion store (D-6, FED-15)
///
/// Only unexpired assertions are ever returned.
///
#[async_trait]
pub trait AssertionStoreRead: Interface + Send + Sync {
    /// The valid assertions of these links
    async fn list_valid_for_links(
        &self,
        link_ids: &[Uuid],
    ) -> Result<Vec<StoredAssertion>, MappedErrors>;

    /// Every valid assertion, or only those from one peer
    async fn list_valid(
        &self,
        peer_id: Option<Uuid>,
    ) -> Result<Vec<StoredAssertion>, MappedErrors>;
}
