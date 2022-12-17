use crate::domain::dtos::account::AccountTypeDTO;

use async_trait::async_trait;
use clean_base::{
    entities::default_response::DeletionResponseKind,
    utils::errors::MappedErrors,
};
use shaku::Interface;

#[async_trait]
pub trait AccountTypeDeletion: Interface + Send + Sync {
    async fn delete(
        &self,
        account_type: AccountTypeDTO,
    ) -> Result<DeletionResponseKind<AccountTypeDTO>, MappedErrors>;
}
