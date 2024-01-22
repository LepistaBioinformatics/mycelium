use crate::domain::dtos::account::AccountType;

use async_trait::async_trait;
use mycelium_base::{
    entities::GetOrCreateResponseKind, utils::errors::MappedErrors,
};
use shaku::Interface;

#[async_trait]
pub trait AccountTypeRegistration: Interface + Send + Sync {
    async fn get_or_create(
        &self,
        account_type: AccountType,
    ) -> Result<GetOrCreateResponseKind<AccountType>, MappedErrors>;
}
