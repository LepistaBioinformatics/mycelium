use crate::domain::dtos::account::AccountTypeDTO;

use agrobase::{
    entities::default_response::{CreateResponseKind, GetOrCreateResponseKind},
    utils::errors::MappedErrors,
};
use async_trait::async_trait;
use shaku::Interface;

#[async_trait]
pub trait AccountTypeRegistration: Interface + Send + Sync {
    async fn get_or_create(
        &self,
        account_type: AccountTypeDTO,
    ) -> Result<GetOrCreateResponseKind<AccountTypeDTO>, MappedErrors>;

    async fn create(
        &self,
        account_type: AccountTypeDTO,
    ) -> Result<CreateResponseKind<AccountTypeDTO>, MappedErrors>;
}
