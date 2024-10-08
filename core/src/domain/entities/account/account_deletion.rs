use async_trait::async_trait;
use mycelium_base::{
    entities::DeletionResponseKind, utils::errors::MappedErrors,
};
use shaku::Interface;
use uuid::Uuid;

use crate::domain::dtos::related_accounts::RelatedAccounts;

#[async_trait]
pub trait AccountDeletion: Interface + Send + Sync {
    async fn delete(
        &self,
        account_id: Uuid,
        related_accounts: RelatedAccounts,
    ) -> Result<DeletionResponseKind<Uuid>, MappedErrors>;
}
