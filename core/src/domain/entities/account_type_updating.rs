use crate::domain::dtos::account::AccountType;

use async_trait::async_trait;
use clean_base::{entities::UpdatingResponseKind, utils::errors::MappedErrors};
use shaku::Interface;

#[async_trait]
pub trait AccountTypeUpdating: Interface + Send + Sync {
    async fn update(
        &self,
        application: AccountType,
    ) -> Result<UpdatingResponseKind<AccountType>, MappedErrors>;
}
