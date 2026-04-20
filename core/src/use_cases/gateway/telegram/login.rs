use super::resolve::resolve_account_by_telegram_id;
use crate::{
    domain::{
        dtos::{telegram::TelegramUser, token::UserAccountScope},
        entities::AccountFetching,
    },
    models::AccountLifeCycle,
};

use chrono::{Duration, Local};
use mycelium_base::utils::errors::{use_case_err, MappedErrors};
use uuid::Uuid;

/// Authenticate via a verified Telegram identity and issue a connection string.
///
/// Cryptographic verification (HMAC) happens in the port handler before this
/// use-case is called — it receives an already-verified `TelegramUser`.
///
/// Returns a `UserAccountScope` connection string that the caller uses for
/// subsequent Mycelium MCP or REST API calls (Mode A authentication).
#[tracing::instrument(
    name = "login_via_telegram",
    skip(account_fetching, config)
)]
pub async fn login_via_telegram(
    tenant_id: Uuid,
    telegram_user: TelegramUser,
    account_fetching: Box<&dyn AccountFetching>,
    config: AccountLifeCycle,
) -> Result<(UserAccountScope, chrono::DateTime<Local>), MappedErrors> {
    let account =
        resolve_account_by_telegram_id(telegram_user.id, account_fetching)
            .await
            .map_err(|e| {
                use_case_err(format!("telegram_id_not_linked: {e}"))
                    .with_exp_true()
            })?;

    let account_id = account.id.ok_or_else(|| {
        use_case_err("Account has no id — data integrity error")
    })?;

    let ttl_secs = config.token_expiration.async_get_or_error().await?;
    let expires_at = Local::now() + Duration::seconds(ttl_secs);

    let connection_string = UserAccountScope::new(
        account_id,
        expires_at,
        None,
        Some(tenant_id),
        None,
        config,
    )
    .await?;

    Ok((connection_string, expires_at))
}
