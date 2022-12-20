use crate::domain::dtos::account::AccountTypeDTO;

use async_trait::async_trait;
use clean_base::{
    entities::default_response::GetOrCreateResponseKind,
    utils::errors::MappedErrors,
};
use shaku::Interface;

#[async_trait]
pub trait AccountTypeRegistration: Interface + Send + Sync {
    async fn get_or_create(
        &self,
        account_type: AccountTypeDTO,
    ) -> Result<GetOrCreateResponseKind<AccountTypeDTO>, MappedErrors>;
}
