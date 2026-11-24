use async_trait::async_trait;
use mycelium_base::utils::errors::MappedErrors;
use shaku::Interface;
use uuid::Uuid;

/// Which ids already name a local tenant or account (I-1, FED-54)
#[async_trait]
pub trait LocalNamespaceProbe: Interface + Send + Sync {
    /// The subset of `ids` that exist locally as a tenant or an account
    async fn existing(&self, ids: &[Uuid]) -> Result<Vec<Uuid>, MappedErrors>;
}
