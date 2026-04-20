use crate::domain::{
    dtos::{account::Account, telegram::TelegramUserId},
    entities::AccountFetching,
};

use mycelium_base::{
    entities::FetchResponseKind,
    utils::errors::{fetching_err, MappedErrors},
};
use uuid::Uuid;

/// Find the account linked to a Telegram identity within a tenant.
///
/// Always requires `tenant_id` — lookup without tenant scope is a bug (see
/// spec OQ-2b: same from.id may exist in multiple tenants).
#[tracing::instrument(
    name = "resolve_account_by_telegram_id",
    skip(account_fetching)
)]
pub async fn resolve_account_by_telegram_id(
    telegram_user_id: TelegramUserId,
    tenant_id: Uuid,
    account_fetching: Box<&dyn AccountFetching>,
) -> Result<Account, MappedErrors> {
    match account_fetching
        .get_by_telegram_id(telegram_user_id, tenant_id)
        .await?
    {
        FetchResponseKind::Found(account) => Ok(account),
        FetchResponseKind::NotFound(_) => {
            fetching_err("telegram_id_not_linked")
                .with_exp_true()
                .as_error()
        }
    }
}
