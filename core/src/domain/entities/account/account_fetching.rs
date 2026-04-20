use crate::domain::dtos::{
    account::Account, account_type::AccountType,
    related_accounts::RelatedAccounts, telegram::TelegramUserId,
};

use async_trait::async_trait;
use mycelium_base::{
    entities::{FetchManyResponseKind, FetchResponseKind},
    utils::errors::MappedErrors,
};
use shaku::Interface;
use uuid::Uuid;

#[async_trait]
pub trait AccountFetching: Interface + Send + Sync {
    async fn get(
        &self,
        id: Uuid,
        related_accounts: RelatedAccounts,
    ) -> Result<FetchResponseKind<Account, Uuid>, MappedErrors>;

    async fn list(
        &self,
        related_accounts: RelatedAccounts,
        term: Option<String>,
        is_owner_active: Option<bool>,
        is_account_active: Option<bool>,
        is_account_checked: Option<bool>,
        is_account_archived: Option<bool>,
        is_account_deleted: Option<bool>,
        tag_id: Option<Uuid>,
        tag_value: Option<String>,
        account_id: Option<Uuid>,
        account_type: AccountType,
        page_size: Option<i32>,
        skip: Option<i32>,
    ) -> Result<FetchManyResponseKind<Account>, MappedErrors>;

    /// Find the personal account linked to a Telegram identity.
    ///
    /// Global lookup — personal accounts (user, manager, staff) have no
    /// `tenant_id`. A Telegram ID maps to at most one account globally.
    /// Uses the GIN index on `account.meta` for the JSONB containment query.
    async fn get_by_telegram_id(
        &self,
        telegram_user_id: TelegramUserId,
    ) -> Result<FetchResponseKind<Account, i64>, MappedErrors>;
}
