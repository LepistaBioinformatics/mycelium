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
    async fn hard_delete(
        &self,
        account_id: Uuid,
        related_accounts: RelatedAccounts,
    ) -> Result<DeletionResponseKind<Uuid>, MappedErrors>;

    /// Soft delete account
    ///
    /// This method marks the account as deleted without removing it from the
    /// database. This method already remove all associated metadata, rename the
    /// slug with the account ID plus deleted suffix.
    ///
    /// This action will prevent the account from being used in the application,
    /// but it will still be present in the database for audit.
    ///
    async fn soft_delete(
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
