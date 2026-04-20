use crate::dtos::{MyceliumProfileData, TenantData};

use actix_web::{post, web, HttpResponse, Responder};
use myc_core::{
    domain::entities::TenantRegistration, models::AccountLifeCycle,
    use_cases::gateway::telegram::set_telegram_config,
};
use myc_diesel::repositories::SqlAppModule;
use myc_http_tools::{
    utils::HttpJsonResponse,
    wrappers::default_response_to_http_response::handle_mapped_error,
};
use serde::Deserialize;
use shaku::HasComponent;
use utoipa::ToSchema;

// ? ---------------------------------------------------------------------------
// ? Configure application
// ? ---------------------------------------------------------------------------

pub fn configure(config: &mut web::ServiceConfig) {
    config.service(set_telegram_config_url);
}

// ? ---------------------------------------------------------------------------
// ? Define API structs
// ? ---------------------------------------------------------------------------

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SetTelegramConfigBody {
    bot_token: String,
    webhook_secret: String,
}

// ? ---------------------------------------------------------------------------
// ? Define API paths
// ? ---------------------------------------------------------------------------

/// Configure Telegram bot credentials for a tenant
///
/// Accepts plain bot token and webhook secret, encrypts both with
/// AES-256-GCM, and stores the ciphertexts in tenant meta. Only the tenant
/// owner may call this endpoint.
///
/// After this call succeeds, `POST /auth/telegram/link` and
/// `POST /auth/telegram/login/{tenant_id}` become operational for the tenant.
///
#[utoipa::path(
    post,
    path = "/tenant-owner/telegram/config",
    operation_id = "set_telegram_config",
    params(
        (
            "x-mycelium-tenant-id" = Uuid,
            Header,
            description = "The tenant unique id.",
        )
    ),
    request_body = SetTelegramConfigBody,
    responses(
        (
            status = 204,
            description = "Telegram credentials configured successfully.",
        ),
        (
            status = 401,
            description = "Unauthorized.",
            body = HttpJsonResponse,
        ),
        (
            status = 403,
            description = "Forbidden — caller is not a tenant owner.",
            body = HttpJsonResponse,
        ),
        (
            status = 500,
            description = "Unknown internal server error.",
            body = HttpJsonResponse,
        ),
    ),
)]
#[post("")]
pub async fn set_telegram_config_url(
    profile: MyceliumProfileData,
    tenant: TenantData,
    body: web::Json<SetTelegramConfigBody>,
    life_cycle_settings: web::Data<AccountLifeCycle>,
    app_module: web::Data<SqlAppModule>,
) -> impl Responder {
    match set_telegram_config(
        profile.to_profile(),
        tenant.tenant_id().to_owned(),
        body.bot_token.clone(),
        body.webhook_secret.clone(),
        life_cycle_settings.get_ref().clone(),
        Box::new(&*app_module.resolve_ref() as &dyn TenantRegistration),
    )
    .await
    {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(err) => handle_mapped_error(err),
    }
}
