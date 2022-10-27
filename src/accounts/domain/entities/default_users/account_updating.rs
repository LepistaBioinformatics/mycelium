use crate::domain::{
    dtos::account::AccountDTO,
    entities::shared::default_responses::UpdateResponse,
    utils::errors::MappedErrors,
};

use async_trait::async_trait;
use shaku::Interface;

#[async_trait]
pub trait UserUpdating: Interface + Send + Sync {
    async fn update(
        &self,
        account: AccountDTO,
    ) -> Result<UpdateResponse<AccountDTO>, MappedErrors>;
}
