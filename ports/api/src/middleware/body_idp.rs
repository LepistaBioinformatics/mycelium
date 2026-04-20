use actix_web::{web, web::Bytes, HttpRequest};
use async_trait::async_trait;
use myc_core::domain::{
    dtos::{
        related_accounts::RelatedAccounts,
        telegram::TelegramUserId,
    },
    entities::AccountFetching,
};
use myc_diesel::repositories::SqlAppModule;
use myc_http_tools::{responses::GatewayError, Email};
use mycelium_base::{dtos::Children, entities::FetchResponseKind};
use shaku::HasComponent;
use uuid::Uuid;

/// Platform-agnostic interface for body-based identity providers.
///
/// Implementations extract a platform user ID from the buffered request body
/// and resolve the linked Mycelium account email. Add a new messaging IdP by:
/// 1. Implementing this trait on a new zero-sized struct.
/// 2. Adding the variant to `build_body_idp_resolver` in `router/mod.rs`.
/// Implementations run on Actix-web single-threaded workers (LocalSet), so
/// `Send` is not required.
#[async_trait(?Send)]
pub(crate) trait BodyIdpResolver {
    fn extract_user_id(&self, body: &Bytes) -> Result<String, GatewayError>;

    async fn resolve_email(
        &self,
        user_id: &str,
        req: &HttpRequest,
    ) -> Result<Email, GatewayError>;
}

/// Bundles the resolver with the already-extracted user ID so a single value
/// travels through the authentication pipeline.
pub(crate) struct BodyIdpContext {
    pub resolver: Box<dyn BodyIdpResolver>,
    pub user_id: String,
}

// ---------------------------------------------------------------------------
// Telegram implementation
// ---------------------------------------------------------------------------

pub(crate) struct TelegramIdpResolver;

#[async_trait(?Send)]
impl BodyIdpResolver for TelegramIdpResolver {
    fn extract_user_id(&self, body: &Bytes) -> Result<String, GatewayError> {
        let value: serde_json::Value =
            serde_json::from_slice(body).map_err(|_| {
                GatewayError::BadRequest(
                    "Request body is not valid JSON".to_string(),
                )
            })?;

        const UPDATE_TYPES: &[&str] = &[
            "message",
            "edited_message",
            "channel_post",
            "edited_channel_post",
            "callback_query",
            "inline_query",
            "shipping_query",
            "pre_checkout_query",
            "poll_answer",
            "my_chat_member",
            "chat_member",
            "chat_join_request",
        ];

        UPDATE_TYPES
            .iter()
            .find_map(|key| value.get(key)?.get("from")?.get("id")?.as_i64())
            .map(|id| id.to_string())
            .ok_or_else(|| {
                GatewayError::Unauthorized(
                    "from.id not found in Telegram update body".to_string(),
                )
            })
    }

    async fn resolve_email(
        &self,
        user_id: &str,
        req: &HttpRequest,
    ) -> Result<Email, GatewayError> {
        let from_id: i64 = user_id.parse().map_err(|_| {
            GatewayError::InternalServerError(
                "Telegram user_id is not a valid i64".to_string(),
            )
        })?;

        let sql_module =
            req.app_data::<web::Data<SqlAppModule>>().ok_or_else(|| {
                GatewayError::InternalServerError(
                    "SQL module not available".to_string(),
                )
            })?;

        let account_fetching: &dyn AccountFetching = sql_module.resolve_ref();

        let account_id = fetch_account_id(account_fetching, from_id).await?;
        fetch_owner_email(account_fetching, account_id).await
    }
}

async fn fetch_account_id(
    account_fetching: &dyn AccountFetching,
    from_id: i64,
) -> Result<Uuid, GatewayError> {
    match account_fetching
        .get_by_telegram_id(TelegramUserId(from_id))
        .await
        .map_err(|e| {
            GatewayError::InternalServerError(format!(
                "Account lookup by telegram_id failed: {e}"
            ))
        })? {
        FetchResponseKind::Found(account) => account.id.ok_or_else(|| {
            GatewayError::InternalServerError(
                "Account resolved but has no id".to_string(),
            )
        }),
        FetchResponseKind::NotFound(_) => Err(GatewayError::Unauthorized(
            "Telegram identity not linked to any account".to_string(),
        )),
    }
}

async fn fetch_owner_email(
    account_fetching: &dyn AccountFetching,
    account_id: Uuid,
) -> Result<Email, GatewayError> {
    let account = match account_fetching
        .get(
            account_id,
            RelatedAccounts::AllowedAccounts(vec![account_id]),
        )
        .await
        .map_err(|e| {
            GatewayError::InternalServerError(format!(
                "Account get-with-owners failed: {e}"
            ))
        })? {
        FetchResponseKind::Found(account) => account,
        FetchResponseKind::NotFound(_) => {
            return Err(GatewayError::InternalServerError(
                "Account not found after id resolution".to_string(),
            ))
        }
    };

    let Children::Records(owners) = account.owners else {
        return Err(GatewayError::InternalServerError(
            "Account owners not loaded".to_string(),
        ));
    };

    owners.into_iter().next().map(|u| u.email).ok_or_else(|| {
        GatewayError::InternalServerError("Account has no owner".to_string())
    })
}
