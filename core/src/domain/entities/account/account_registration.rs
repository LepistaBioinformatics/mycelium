use crate::domain::dtos::account::Account;

use async_trait::async_trait;
use mycelium_base::{
    entities::{CreateResponseKind, GetOrCreateResponseKind},
    utils::errors::MappedErrors,
};
use shaku::Interface;
use uuid::Uuid;

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

    async fn create_subscription_account(
        &self,
        account: Account,
        tenant_id: Uuid,
    ) -> Result<CreateResponseKind<Account>, MappedErrors>;
}
