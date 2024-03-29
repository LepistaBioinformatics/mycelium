use crate::domain::dtos::account::AccountType;

use async_trait::async_trait;
use mycelium_base::{
    entities::UpdatingResponseKind, utils::errors::MappedErrors,
};
use shaku::Interface;

#[async_trait]
pub trait AccountTypeUpdating: Interface + Send + Sync {
    async fn update(
        &self,
        account_type: AccountType,
    ) -> Result<UpdatingResponseKind<AccountType>, MappedErrors>;
}
