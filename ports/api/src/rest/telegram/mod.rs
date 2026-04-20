use crate::dtos::{MyceliumProfileData, TenantData};

use actix_web::{delete, post, web, HttpRequest, HttpResponse, Responder};
use chrono::Utc;
use myc_core::{
    domain::entities::{
        AccountDeletion, AccountFetching, AccountUpdating, TelegramConfig,
        TenantFetching,
    },
    models::AccountLifeCycle,
    use_cases::gateway::telegram::{
        link_telegram_identity, login_via_telegram, unlink_telegram_identity,
    },
};
use myc_diesel::repositories::SqlAppModule;
use myc_http_tools::{
    telegram::{
        types::{BotToken, InitData, WebhookSecret},
        verify_init_data, verify_webhook_secret,
    },
    utils::HttpJsonResponse,
    wrappers::default_response_to_http_response::handle_mapped_error,
};
use myc_svc::repositories::TelegramConfigSvcRepo;
use mycelium_base::entities::FetchResponseKind;
use secrecy::SecretString;
use serde::Serialize;
use shaku::HasComponent;
use tracing::warn;
use uuid::Uuid;

// ? ---------------------------------------------------------------------------
// ? Configure application
// ? ---------------------------------------------------------------------------

pub fn configure(config: &mut web::ServiceConfig) {
    config
        .service(link_telegram_url)
        .service(unlink_telegram_url)
        .service(login_via_telegram_url)
        .service(webhook_url);
}

// ? ---------------------------------------------------------------------------
// ? Define API structs
// ? ---------------------------------------------------------------------------

#[derive(serde::Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TelegramInitDataBody {
    init_data: String,
}

#[derive(Serialize, utoipa::ToResponse, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TelegramLoginResponse {
    connection_string: String,
    expires_at: chrono::DateTime<chrono::Local>,
}

// ? ---------------------------------------------------------------------------
// ? Define API paths
// ? ---------------------------------------------------------------------------

/// Link Telegram identity
///
/// Verifies the Telegram Mini App initData HMAC and stores the Telegram user
/// identifier in the authenticated Mycelium account's metadata. Requires a
/// valid connection-string or JWT in the request.
///
/// **Cross-tenant constraint**: The authenticated account (`x-mycelium-profile`)
/// must be a guest or subscriber of the tenant supplied in
/// `x-mycelium-tenant-id`. If it is not, the link will be stored but
/// `POST /auth/telegram/login/{tenant_id}` will return 404 for that account
/// because the tenant-scoped lookup will find no matching guest record. This
/// constraint is enforced by the `login_via_telegram` use-case rather than
/// here; callers must ensure the account belongs to the target tenant.
///
#[utoipa::path(
    post,
    path = "/auth/telegram/link",
    operation_id = "link_telegram_identity",
    request_body = TelegramInitDataBody,
    responses(
        (
            status = 204,
            description = "Telegram identity linked successfully.",
        ),
        (
            status = 401,
            description = "Unauthorized.",
            body = HttpJsonResponse,
        ),
        (
            status = 409,
            description = "Conflict — already linked or Telegram ID in use.",
            body = HttpJsonResponse,
        ),
        (
            status = 422,
            description = "Telegram not configured for this tenant.",
            body = HttpJsonResponse,
        ),
        (
            status = 500,
            description = "Unknown internal server error.",
            body = HttpJsonResponse,
        ),
    ),
)]
#[post("/link")]
pub async fn link_telegram_url(
    profile: MyceliumProfileData,
    tenant: TenantData,
    body: web::Json<TelegramInitDataBody>,
    life_cycle_settings: web::Data<AccountLifeCycle>,
    sql_app_module: web::Data<SqlAppModule>,
) -> impl Responder {
    let tenant_id = *tenant.tenant_id();

    let tenant_record = match fetch_tenant(tenant_id, &sql_app_module).await {
        Ok(t) => t,
        Err(resp) => return resp,
    };

    let meta = match tenant_record.meta {
        Some(m) => m,
        None => {
            return HttpResponse::UnprocessableEntity().json(
                HttpJsonResponse::new_message(
                    "telegram_not_configured_for_tenant",
                ),
            )
        }
    };

    let config_repo = match TelegramConfigSvcRepo::from_tenant_meta(
        &meta,
        life_cycle_settings.get_ref().clone(),
    )
    .await
    {
        Ok(r) => r,
        Err(err) => {
            warn!("telegram config missing: {:?}", err);
            return HttpResponse::UnprocessableEntity().json(
                HttpJsonResponse::new_message(
                    "telegram_not_configured_for_tenant",
                ),
            );
        }
    };

    let bot_token_str: String = match config_repo.get_bot_token(tenant_id).await
    {
        Ok(t) => t,
        Err(err) => {
            warn!("failed to resolve bot token: {:?}", err);
            return HttpResponse::InternalServerError().json(
                HttpJsonResponse::new_message("failed_to_resolve_bot_token"),
            );
        }
    };

    let bot_token = BotToken(SecretString::new(bot_token_str.into()));

    let telegram_user = match verify_init_data(
        &InitData(body.init_data.clone()),
        &bot_token,
        Utc::now(),
    ) {
        Ok(u) => u,
        Err(err) => {
            warn!("telegram initData verification failed: {:?}", err);
            return HttpResponse::Unauthorized().json(
                HttpJsonResponse::new_message("invalid_telegram_init_data"),
            );
        }
    };

    match link_telegram_identity(
        profile.acc_id,
        tenant_id,
        telegram_user,
        Box::new(&*sql_app_module.resolve_ref() as &dyn AccountFetching),
        Box::new(&*sql_app_module.resolve_ref() as &dyn AccountUpdating),
    )
    .await
    {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(err) => handle_mapped_error(err),
    }
}

/// Unlink Telegram identity
///
/// Removes the Telegram identifier from the authenticated Mycelium account's
/// metadata.
///
#[utoipa::path(
    delete,
    path = "/auth/telegram/link",
    operation_id = "unlink_telegram_identity",
    responses(
        (
            status = 204,
            description = "Telegram identity unlinked successfully.",
        ),
        (
            status = 401,
            description = "Unauthorized.",
            body = HttpJsonResponse,
        ),
        (
            status = 404,
            description = "Telegram identity not linked.",
            body = HttpJsonResponse,
        ),
        (
            status = 500,
            description = "Unknown internal server error.",
            body = HttpJsonResponse,
        ),
    ),
)]
#[delete("/link")]
pub async fn unlink_telegram_url(
    profile: MyceliumProfileData,
    sql_app_module: web::Data<SqlAppModule>,
) -> impl Responder {
    match unlink_telegram_identity(
        profile.acc_id,
        Box::new(&*sql_app_module.resolve_ref() as &dyn AccountFetching),
        Box::new(&*sql_app_module.resolve_ref() as &dyn AccountDeletion),
    )
    .await
    {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(err) => handle_mapped_error(err),
    }
}

/// Login via Telegram
///
/// Verifies Telegram Mini App initData for the given tenant, resolves the
/// linked Mycelium account, and returns a connection string for subsequent
/// authenticated calls.
///
#[utoipa::path(
    post,
    path = "/auth/telegram/login/{tenant_id}",
    operation_id = "login_via_telegram",
    params(
        (
            "tenant_id" = Uuid,
            Path,
            description = "Tenant UUID that owns this Telegram bot.",
        )
    ),
    request_body = TelegramInitDataBody,
    responses(
        (
            status = 200,
            description = "Login successful — connection string issued.",
            body = TelegramLoginResponse,
        ),
        (
            status = 401,
            description = "Unauthorized — invalid or expired initData.",
            body = HttpJsonResponse,
        ),
        (
            status = 404,
            description = "Telegram ID not linked to any account in this tenant.",
            body = HttpJsonResponse,
        ),
        (
            status = 422,
            description = "Telegram not configured for this tenant.",
            body = HttpJsonResponse,
        ),
        (
            status = 500,
            description = "Unknown internal server error.",
            body = HttpJsonResponse,
        ),
    ),
    security(()),
)]
#[post("/login/{tenant_id}")]
pub async fn login_via_telegram_url(
    tenant_id: web::Path<Uuid>,
    body: web::Json<TelegramInitDataBody>,
    life_cycle_settings: web::Data<AccountLifeCycle>,
    sql_app_module: web::Data<SqlAppModule>,
) -> impl Responder {
    let tenant_id = tenant_id.into_inner();

    let tenant_record = match fetch_tenant(tenant_id, &sql_app_module).await {
        Ok(t) => t,
        Err(resp) => return resp,
    };

    let meta = match tenant_record.meta {
        Some(m) => m,
        None => {
            return HttpResponse::UnprocessableEntity().json(
                HttpJsonResponse::new_message(
                    "telegram_not_configured_for_tenant",
                ),
            )
        }
    };

    let config_repo = match TelegramConfigSvcRepo::from_tenant_meta(
        &meta,
        life_cycle_settings.get_ref().clone(),
    )
    .await
    {
        Ok(r) => r,
        Err(err) => {
            warn!("telegram config missing: {:?}", err);
            return HttpResponse::UnprocessableEntity().json(
                HttpJsonResponse::new_message(
                    "telegram_not_configured_for_tenant",
                ),
            );
        }
    };

    let bot_token_str: String = match config_repo.get_bot_token(tenant_id).await
    {
        Ok(t) => t,
        Err(err) => {
            warn!("failed to resolve bot token: {:?}", err);
            return HttpResponse::InternalServerError().json(
                HttpJsonResponse::new_message("failed_to_resolve_bot_token"),
            );
        }
    };

    let bot_token = BotToken(SecretString::new(bot_token_str.into()));

    let telegram_user = match verify_init_data(
        &InitData(body.init_data.clone()),
        &bot_token,
        Utc::now(),
    ) {
        Ok(u) => u,
        Err(err) => {
            warn!("telegram initData verification failed: {:?}", err);
            return HttpResponse::Unauthorized().json(
                HttpJsonResponse::new_message("invalid_telegram_init_data"),
            );
        }
    };

    match login_via_telegram(
        tenant_id,
        telegram_user,
        Box::new(&*sql_app_module.resolve_ref() as &dyn AccountFetching),
        life_cycle_settings.get_ref().to_owned(),
    )
    .await
    {
        Ok((connection_string, expires_at)) => {
            HttpResponse::Ok().json(TelegramLoginResponse {
                connection_string: connection_string.to_string(),
                expires_at,
            })
        }
        Err(err) => handle_mapped_error(err),
    }
}

/// Telegram webhook
///
/// Receives Telegram update callbacks for the given tenant. Verifies the
/// `X-Telegram-Bot-Api-Secret-Token` header before accepting. Returns `200 OK`
/// immediately — Telegram retries on any other status code.
///
/// The update payload is accepted and validated here; forwarding to downstream
/// handlers is wired in T19 (Mode B routing).
///
#[utoipa::path(
    post,
    path = "/auth/telegram/webhook/{tenant_id}",
    operation_id = "telegram_webhook",
    params(
        (
            "tenant_id" = Uuid,
            Path,
            description = "Tenant UUID that owns this Telegram bot.",
        )
    ),
    request_body = serde_json::Value,
    responses(
        (
            status = 200,
            description = "Update accepted.",
        ),
        (
            status = 401,
            description = "Invalid webhook secret.",
            body = HttpJsonResponse,
        ),
        (
            status = 422,
            description = "Telegram not configured for this tenant.",
            body = HttpJsonResponse,
        ),
        (
            status = 500,
            description = "Unknown internal server error.",
            body = HttpJsonResponse,
        ),
    ),
    security(()),
)]
#[post("/webhook/{tenant_id}")]
pub async fn webhook_url(
    req: HttpRequest,
    tenant_id: web::Path<Uuid>,
    _body: web::Json<serde_json::Value>,
    life_cycle_settings: web::Data<AccountLifeCycle>,
    sql_app_module: web::Data<SqlAppModule>,
) -> impl Responder {
    let tenant_id = tenant_id.into_inner();

    let tenant_record = match fetch_tenant(tenant_id, &sql_app_module).await {
        Ok(t) => t,
        Err(resp) => return resp,
    };

    let meta = match tenant_record.meta {
        Some(m) => m,
        None => {
            return HttpResponse::UnprocessableEntity().json(
                HttpJsonResponse::new_message(
                    "telegram_not_configured_for_tenant",
                ),
            )
        }
    };

    let config_repo = match TelegramConfigSvcRepo::from_tenant_meta(
        &meta,
        life_cycle_settings.get_ref().clone(),
    )
    .await
    {
        Ok(r) => r,
        Err(err) => {
            warn!("telegram config missing: {:?}", err);
            return HttpResponse::UnprocessableEntity().json(
                HttpJsonResponse::new_message(
                    "telegram_not_configured_for_tenant",
                ),
            );
        }
    };

    let webhook_secret_str: String =
        match config_repo.get_webhook_secret(tenant_id).await {
            Ok(s) => s,
            Err(err) => {
                warn!("failed to resolve webhook secret: {:?}", err);
                return HttpResponse::InternalServerError().json(
                    HttpJsonResponse::new_message(
                        "failed_to_resolve_webhook_secret",
                    ),
                );
            }
        };

    let expected_secret =
        WebhookSecret(SecretString::new(webhook_secret_str.into()));

    let header_value = req
        .headers()
        .get("X-Telegram-Bot-Api-Secret-Token")
        .and_then(|v| v.to_str().ok());

    if !verify_webhook_secret(header_value, &expected_secret) {
        return HttpResponse::Unauthorized()
            .json(HttpJsonResponse::new_message("invalid_webhook_secret"));
    }

    tracing::info!(
        tenant_id = %tenant_id,
        "telegram_webhook_received"
    );

    HttpResponse::Ok().finish()
}

// ? ---------------------------------------------------------------------------
// ? Private helpers
// ? ---------------------------------------------------------------------------

async fn fetch_tenant(
    tenant_id: Uuid,
    sql_app_module: &SqlAppModule,
) -> Result<myc_core::domain::dtos::tenant::Tenant, HttpResponse> {
    let repo: &dyn TenantFetching = sql_app_module.resolve_ref();

    match repo.get_tenant_public_by_id(tenant_id).await {
        Ok(FetchResponseKind::Found(t)) => Ok(t),
        Ok(FetchResponseKind::NotFound(_)) => Err(HttpResponse::NotFound()
            .json(HttpJsonResponse::new_message("tenant_not_found"))),
        Err(err) => {
            warn!("failed to fetch tenant: {:?}", err);
            Err(HttpResponse::InternalServerError()
                .json(HttpJsonResponse::new_message("failed_to_fetch_tenant")))
        }
    }
}
