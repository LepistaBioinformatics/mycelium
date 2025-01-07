use crate::domain::dtos::{account::Account, account_type::AccountType};

use async_trait::async_trait;
use mycelium_base::{
    entities::UpdatingResponseKind, utils::errors::MappedErrors,
};
use shaku::Interface;
use uuid::Uuid;

#[async_trait]
pub trait AccountUpdating: Interface + Send + Sync {
    async fn update(
        &self,
        account: Account,
    ) -> Result<UpdatingResponseKind<Account>, MappedErrors>;

    async fn update_own_account_name(
        &self,
        account_id: Uuid,
        name: String,
    ) -> Result<UpdatingResponseKind<Account>, MappedErrors>;

    async fn update_account_type(
        &self,
        account_id: Uuid,
        account_type: AccountType,
    ) -> Result<UpdatingResponseKind<Account>, MappedErrors>;
}
