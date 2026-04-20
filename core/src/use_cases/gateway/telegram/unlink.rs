use crate::domain::{
    dtos::account::AccountMetaKey,
    entities::{AccountDeletion, AccountFetching},
};

use mycelium_base::{
    entities::{DeletionResponseKind, FetchResponseKind},
    utils::errors::{fetching_err, use_case_err, MappedErrors},
};
use uuid::Uuid;

/// Remove a Telegram identity link from a Mycelium account.
#[tracing::instrument(
    name = "unlink_telegram_identity",
    skip(account_fetching, account_deletion)
)]
pub async fn unlink_telegram_identity(
    account_id: Uuid,
    account_fetching: Box<&dyn AccountFetching>,
    account_deletion: Box<&dyn AccountDeletion>,
) -> Result<(), MappedErrors> {
    let account = match account_fetching
        .get(
            account_id,
            crate::domain::dtos::related_accounts::RelatedAccounts::AllowedAccounts(
                vec![account_id],
            ),
        )
        .await?
    {
        FetchResponseKind::Found(a) => a,
        FetchResponseKind::NotFound(_) => {
            return fetching_err("account_not_found")
                .with_exp_true()
                .as_error()
        }
    };

    let has_link = account
        .meta
        .as_ref()
        .map(|m| m.contains_key(&AccountMetaKey::TelegramUser))
        .unwrap_or(false);

    if !has_link {
        return fetching_err("telegram_not_linked")
            .with_exp_true()
            .as_error();
    }

    match account_deletion
        .delete_account_meta(account_id, AccountMetaKey::TelegramUser)
        .await?
    {
        DeletionResponseKind::Deleted => Ok(()),
        DeletionResponseKind::NotDeleted(_, msg) => {
            use_case_err(format!("Failed to unlink telegram identity: {msg}"))
                .as_error()
        }
    }
}
