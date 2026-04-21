use async_trait::async_trait;
use mycelium_base::utils::errors::MappedErrors;
use shaku::Interface;
use uuid::Uuid;

/// Port for fetching or provisioning per-tenant data-encryption keys (DEKs).
///
/// Implementations must fetch the tenant's wrapped DEK from the database,
/// unwrap it with the supplied KEK, and return the plaintext DEK.  When a
/// tenant has no DEK yet, the implementation generates one, wraps it, and
/// persists it before returning.
///
/// `tenant_id = None` addresses the system DEK used for accounts that have no
/// tenant affiliation (e.g. Staff).
#[async_trait]
pub trait EncryptionKeyFetching: Interface + Send + Sync {
    async fn get_or_provision_dek(
        &self,
        tenant_id: Option<Uuid>,
        kek: &[u8; 32],
    ) -> Result<[u8; 32], MappedErrors>;
}
