use crate::domain::dtos::account::AccountDTO;

use agrobase::{
    entities::default_response::UpdatingResponseKind,
    utils::errors::MappedErrors,
};
use async_trait::async_trait;
use shaku::Interface;

#[async_trait]
pub trait AccountUpdating: Interface + Send + Sync {
    async fn update(
        &self,
        account: AccountDTO,
    ) -> Result<UpdatingResponseKind<AccountDTO>, MappedErrors>;
}
