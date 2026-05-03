use actix_web::{get, web, HttpResponse, Responder};
use myc_core::models::AccountLifeCycle;
use serde::Serialize;
use utoipa::{ToResponse, ToSchema};

// ? ---------------------------------------------------------------------------
// ? Configure application
// ? ---------------------------------------------------------------------------

pub fn configure(config: &mut web::ServiceConfig) {
    config.service(get_app_public_config_url);
}

// ? ---------------------------------------------------------------------------
// ? Define API structs
// ? ---------------------------------------------------------------------------

#[derive(Serialize, ToSchema, ToResponse)]
#[serde(rename_all = "camelCase")]
pub struct AppPublicConfigResponse {
    pub domain_name: String,
    pub domain_url: Option<String>,
    pub locale: Option<String>,
}

// ? ---------------------------------------------------------------------------
// ? Define API paths
// ? ---------------------------------------------------------------------------

/// Get public application configuration
///
/// Returns the application's public branding and locale settings:
/// domain name, optional domain URL, and optional default locale.
/// No authentication required.
#[utoipa::path(
    get,
    operation_id = "get_app_public_config",
    context_path = "/app-config",
    responses(
        (
            status = 200,
            description = "Application public configuration.",
            body = AppPublicConfigResponse,
        ),
        (
            status = 500,
            description = "Configuration resolution failed.",
        ),
    ),
    security(()),
)]
#[get("")]
pub async fn get_app_public_config_url(
    life_cycle_settings: web::Data<AccountLifeCycle>,
) -> impl Responder {
    let domain_name =
        match life_cycle_settings.domain_name.async_get_or_error().await {
            Ok(name) => name,
            Err(_) => return HttpResponse::InternalServerError().finish(),
        };

    let domain_url = match &life_cycle_settings.domain_url {
        Some(resolver) => resolver.async_get_or_error().await.ok(),
        None => None,
    };

    let locale = match &life_cycle_settings.locale {
        Some(resolver) => resolver.async_get_or_error().await.ok(),
        None => None,
    };

    HttpResponse::Ok().json(AppPublicConfigResponse {
        domain_name,
        domain_url,
        locale,
    })
}
