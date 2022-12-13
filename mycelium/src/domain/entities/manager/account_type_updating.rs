use crate::domain::dtos::account::AccountTypeDTO;

use agrobase::{
    entities::default_response::UpdatingResponseKind,
    utils::errors::MappedErrors,
};
use async_trait::async_trait;
use shaku::Interface;

#[async_trait]
pub trait AccountTypeUpdating: Interface + Send + Sync {
    async fn update(
        &self,
        application: AccountTypeDTO,
    ) -> Result<UpdatingResponseKind<AccountTypeDTO>, MappedErrors>;
}
