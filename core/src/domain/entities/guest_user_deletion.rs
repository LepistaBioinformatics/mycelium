use async_trait::async_trait;
use clean_base::{entities::DeletionResponseKind, utils::errors::MappedErrors};
use shaku::Interface;
use uuid::Uuid;

#[async_trait]
pub trait GuestUserDeletion: Interface + Send + Sync {
    async fn delete(
        &self,
        guest_user_id: Uuid,
        account_id: Uuid,
    ) -> Result<DeletionResponseKind<(Uuid, Uuid)>, MappedErrors>;
}
