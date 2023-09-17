use async_trait::async_trait;
use clean_base::{entities::DeletionResponseKind, utils::errors::MappedErrors};
use shaku::Interface;
use uuid::Uuid;

#[async_trait]
pub trait GuestUserDeletion: Interface + Send + Sync {
    async fn delete(
        &self,
        guest_role_id: Uuid,
        account_id: Uuid,
        email: String,
    ) -> Result<DeletionResponseKind<(Uuid, Uuid)>, MappedErrors>;
}
