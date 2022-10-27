use crate::domain::{
    dtos::account::AccountDTO,
    entities::shared::default_responses::{
        CreateResponse, GetOrCreateResponse,
    },
    utils::errors::MappedErrors,
};

use async_trait::async_trait;
use shaku::Interface;

#[async_trait]
pub trait AccountRegistration: Interface + Send + Sync {
    async fn get_or_create(
        &self,
        account: AccountDTO,
    ) -> Result<GetOrCreateResponse<AccountDTO>, MappedErrors>;

    async fn create(
        &self,
        user: AccountDTO,
    ) -> Result<CreateResponse<AccountDTO>, MappedErrors>;
}
