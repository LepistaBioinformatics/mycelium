use crate::domain::dtos::{
    account::AccountMetaKey, related_accounts::RelatedAccounts,
};

use async_trait::async_trait;
use mycelium_base::{
    entities::DeletionResponseKind, utils::errors::MappedErrors,
};
use shaku::Interface;
use uuid::Uuid;

#[async_trait]
pub trait AccountDeletion: Interface + Send + Sync {
    async fn delete(
        &self,
        account_id: Uuid,
        related_accounts: RelatedAccounts,
    ) -> Result<DeletionResponseKind<Uuid>, MappedErrors>;

    async fn delete_account_meta(
        &self,
        account_id: Uuid,
        key: AccountMetaKey,
    ) -> Result<DeletionResponseKind<Uuid>, MappedErrors>;
}
