use crate::domain::dtos::federation::NamespaceClaim;

use async_trait::async_trait;
use mycelium_base::utils::errors::MappedErrors;
use shaku::Interface;
use uuid::Uuid;

/// Claims of foreign ids by peers (I-1, FED-54)
#[async_trait]
pub trait NamespaceClaimRegistration: Interface + Send + Sync {
    /// Record the claim unless the id is already claimed; returns the peer
    /// that owns the id afterwards, which is the first claimant
    async fn claim(&self, claim: NamespaceClaim) -> Result<Uuid, MappedErrors>;
}
