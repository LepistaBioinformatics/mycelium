use crate::domain::{
    dtos::account::AccountTypeDTO,
    entities::shared::default_responses::{
        CreateResponse, GetOrCreateResponse,
    },
    utils::errors::MappedErrors,
};

use async_trait::async_trait;
use shaku::Interface;

#[async_trait]
pub trait AccountTypeRegistration: Interface + Send + Sync {
    async fn get_or_create(
        &self,
        account_type: AccountTypeDTO,
    ) -> Result<GetOrCreateResponse<AccountTypeDTO>, MappedErrors>;

    async fn create(
        &self,
        account_type: AccountTypeDTO,
    ) -> Result<CreateResponse<AccountTypeDTO>, MappedErrors>;
}
