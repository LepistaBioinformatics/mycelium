use crate::domain::dtos::account::Account;

use async_trait::async_trait;
use mycelium_base::{
    entities::UpdatingResponseKind, utils::errors::MappedErrors,
};
use shaku::Interface;

#[async_trait]
pub trait AccountUpdating: Interface + Send + Sync {
    async fn update(
        &self,
        account: Account,
    ) -> Result<UpdatingResponseKind<Account>, MappedErrors>;
}
