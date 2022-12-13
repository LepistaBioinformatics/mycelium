use crate::domain::dtos::account::AccountDTO;

use agrobase::{
    entities::default_response::{CreateResponseKind, GetOrCreateResponseKind},
    utils::errors::MappedErrors,
};
use async_trait::async_trait;
use shaku::Interface;

#[async_trait]
pub trait AccountRegistration: Interface + Send + Sync {
    async fn get_or_create(
        &self,
        account: AccountDTO,
    ) -> Result<GetOrCreateResponseKind<AccountDTO>, MappedErrors>;

    async fn create(
        &self,
        user: AccountDTO,
    ) -> Result<CreateResponseKind<AccountDTO>, MappedErrors>;
}
