use crate::domain::{
    dtos::{account::Account, telegram::TelegramUserId},
    entities::AccountFetching,
};

use mycelium_base::{
    entities::FetchResponseKind,
    utils::errors::{fetching_err, MappedErrors},
};

/// Find the personal account linked to a Telegram identity.
///
/// Global lookup — a Telegram ID maps to at most one personal account.
#[tracing::instrument(
    name = "resolve_account_by_telegram_id",
    skip(account_fetching)
)]
pub async fn resolve_account_by_telegram_id(
    telegram_user_id: TelegramUserId,
    account_fetching: Box<&dyn AccountFetching>,
) -> Result<Account, MappedErrors> {
    match account_fetching
        .get_by_telegram_id(telegram_user_id)
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
