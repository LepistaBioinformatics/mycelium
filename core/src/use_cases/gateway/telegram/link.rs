use crate::domain::{
    dtos::{account::AccountMetaKey, telegram::TelegramUser},
    entities::{AccountFetching, AccountUpdating},
};

use mycelium_base::{
    entities::{FetchResponseKind, UpdatingResponseKind},
    utils::errors::{use_case_err, MappedErrors},
};
use uuid::Uuid;

/// Link a verified Telegram identity to a personal Mycelium account.
///
/// Cryptographic verification (HMAC) is performed by the port handler before
/// calling this use-case — it receives an already-verified `TelegramUser`.
///
/// Steps:
/// 1. Fetch the personal account — 404 if not found.
/// 2. Check account already linked → 409 (already_linked).
/// 3. Check telegram_id linked to any other account globally → 409 (telegram_id_already_used).
/// 4. Write telegram_user to account.meta.
#[tracing::instrument(
    name = "link_telegram_identity",
    skip(account_fetching, account_updating)
)]
pub async fn link_telegram_identity(
    account_id: Uuid,
    telegram_user: TelegramUser,
    account_fetching: Box<&dyn AccountFetching>,
    account_updating: Box<&dyn AccountUpdating>,
) -> Result<(), MappedErrors> {
    let account = account_fetching
        .get(
            account_id,
            crate::domain::dtos::related_accounts::RelatedAccounts::AllowedAccounts(
                vec![account_id],
            ),
        )
        .await?;

    let account = match account {
        FetchResponseKind::Found(a) => a,
        FetchResponseKind::NotFound(_) => {
            return use_case_err("account_not_found").with_exp_true().as_error()
        }
    };

    if account
        .meta
        .as_ref()
        .map(|m| m.contains_key(&AccountMetaKey::TelegramUser))
        .unwrap_or(false)
    {
        return use_case_err("already_linked").with_exp_true().as_error();
    }

    match account_fetching
        .get_by_telegram_id(telegram_user.id.clone())
        .await?
    {
        FetchResponseKind::Found(_) => {
            return use_case_err("telegram_id_already_used")
                .with_exp_true()
                .as_error()
        }
        FetchResponseKind::NotFound(_) => {}
    }

    let meta_value = serde_json::to_string(&TelegramUser {
        id: telegram_user.id,
        username: telegram_user.username,
    })
    .map_err(|e| {
        use_case_err(format!("Failed to serialize telegram user: {e}"))
    })?;

    match account_updating
        .update_account_meta(
            account_id,
            AccountMetaKey::TelegramUser,
            meta_value,
        )
        .await?
    {
        UpdatingResponseKind::Updated(_) => Ok(()),
        UpdatingResponseKind::NotUpdated(_, msg) => {
            use_case_err(format!("Failed to link telegram identity: {msg}"))
                .as_error()
        }
    }
}
