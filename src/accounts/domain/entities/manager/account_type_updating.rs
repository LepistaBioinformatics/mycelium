use crate::domain::{
    dtos::account::AccountTypeDTO,
    entities::shared::default_responses::UpdateResponse,
    utils::errors::MappedErrors,
};

use async_trait::async_trait;
use shaku::Interface;

#[async_trait]
pub trait AccountTypeUpdating: Interface + Send + Sync {
    async fn update(
        &self,
        application: AccountTypeDTO,
    ) -> Result<UpdateResponse<AccountTypeDTO>, MappedErrors>;
}
