use crate::domain::dtos::account::Account;

use async_trait::async_trait;
use clean_base::{
    entities::{CreateResponseKind, GetOrCreateResponseKind},
    utils::errors::MappedErrors,
};
use shaku::Interface;

#[async_trait]
pub trait AccountRegistration: Interface + Send + Sync {
    async fn get_or_create(
        &self,
        account: Account,
        user_exists: bool,
        omit_user_creation: bool,
    ) -> Result<GetOrCreateResponseKind<Account>, MappedErrors>;

    async fn create(
        &self,
        user: Account,
    ) -> Result<CreateResponseKind<Account>, MappedErrors>;
}
