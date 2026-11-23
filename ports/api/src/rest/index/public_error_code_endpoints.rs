use crate::{
    models::active_backend_modules::SqlAppModule,
    rest::shared::PaginationParams,
};

use actix_web::{get, web, Responder};
use myc_core::{
    domain::dtos::error_code::ErrorCode,
    use_cases::gateway::error_codes::list_public_error_codes,
};
use myc_http_tools::{
    utils::HttpJsonResponse,
    wrappers::default_response_to_http_response::{
        fetch_many_response_kind, handle_mapped_error,
    },
};
use serde::Deserialize;
use shaku::HasComponent;
use utoipa::{IntoParams, ToSchema};

// ? ---------------------------------------------------------------------------
// ? Configure application
// ? ---------------------------------------------------------------------------

pub fn configure(config: &mut web::ServiceConfig) {
    config.service(list_public_error_codes_url);
}

// ? ---------------------------------------------------------------------------
// ? Define API structs
// ? ---------------------------------------------------------------------------

#[derive(Deserialize, ToSchema, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct ListPublicErrorCodesParams {
    /// Filter by the error prefix (e.g. `MYC`)
    prefix: Option<String>,

    /// Filter by the error number (e.g. `23` for `MYC00023`)
    code: Option<i32>,
}

// ? ---------------------------------------------------------------------------
// ? Define API paths
// ? ---------------------------------------------------------------------------

/// List public error codes
///
/// Lists the error codes meant to be shown to callers (`isInternal = false`).
/// Internal error codes are never returned. The page size is capped at 100.
/// No authentication required.
#[utoipa::path(
    get,
    operation_id = "list_public_error_codes",
    context_path = "/error-codes",
    params(
        ListPublicErrorCodesParams,
        PaginationParams,
    ),
    responses(
        (
            status = 500,
            description = "Unknown internal server error.",
            body = HttpJsonResponse,
        ),
        (
            status = 204,
            description = "Not found.",
        ),
        (
            status = 200,
            description = "Fetching success.",
            body = [ErrorCode],
        ),
    ),
    security(()),
)]
#[get("")]
pub async fn list_public_error_codes_url(
    info: web::Query<ListPublicErrorCodesParams>,
    page: web::Query<PaginationParams>,
    app_module: web::Data<SqlAppModule>,
) -> impl Responder {
    match list_public_error_codes(
        info.prefix.to_owned(),
        info.code.to_owned(),
        page.page_size.to_owned(),
        page.skip.to_owned(),
        Box::new(&*app_module.resolve_ref()),
    )
    .await
    {
        Ok(res) => fetch_many_response_kind(res),
        Err(err) => handle_mapped_error(err),
    }
}
