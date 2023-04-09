use crate::domain::dtos::account::AccountType;

use async_trait::async_trait;
use clean_base::{entities::DeletionResponseKind, utils::errors::MappedErrors};
use shaku::Interface;

#[async_trait]
pub trait AccountTypeDeletion: Interface + Send + Sync {
    async fn delete(
        &self,
        account_type: AccountType,
    ) -> Result<DeletionResponseKind<AccountType>, MappedErrors>;
}
