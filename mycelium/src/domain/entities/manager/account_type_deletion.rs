use crate::domain::{
    dtos::account::AccountTypeDTO,
    entities::shared::default_responses::DeleteResponse,
    utils::errors::MappedErrors,
};

use async_trait::async_trait;
use shaku::Interface;

#[async_trait]
pub trait AccountTypeDeletion: Interface + Send + Sync {
    async fn delete(
        &self,
        account_type: AccountTypeDTO,
    ) -> Result<DeleteResponse<AccountTypeDTO>, MappedErrors>;
}
