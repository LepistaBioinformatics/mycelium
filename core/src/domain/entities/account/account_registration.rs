use std::collections::HashMap;

use crate::domain::dtos::account::{Account, AccountMetaKey};

use async_trait::async_trait;
use mycelium_base::{
    entities::{CreateResponseKind, GetOrCreateResponseKind},
    utils::errors::MappedErrors,
};
use shaku::Interface;
use uuid::Uuid;

#[async_trait]
pub trait AccountRegistration: Interface + Send + Sync {
    async fn get_or_create_user_account(
        &self,
        account: Account,
        user_exists: bool,
        omit_user_creation: bool,
    ) -> Result<GetOrCreateResponseKind<Account>, MappedErrors>;

    async fn create_subscription_account(
        &self,
        account: Account,
        tenant_id: Uuid,
    ) -> Result<CreateResponseKind<Account>, MappedErrors>;

    async fn get_or_create_tenant_management_account(
        &self,
        account: Account,
        tenant_id: Uuid,
    ) -> Result<GetOrCreateResponseKind<Account>, MappedErrors>;

    async fn get_or_create_role_related_account(
        &self,
        account: Account,
    ) -> Result<GetOrCreateResponseKind<Account>, MappedErrors>;

    async fn get_or_create_actor_related_account(
        &self,
        account: Account,
    ) -> Result<GetOrCreateResponseKind<Account>, MappedErrors>;

    async fn register_account_meta(
        &self,
        account_id: Uuid,
        key: AccountMetaKey,
        value: String,
    ) -> Result<CreateResponseKind<HashMap<String, String>>, MappedErrors>;
}
