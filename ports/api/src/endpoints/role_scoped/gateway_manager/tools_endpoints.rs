use crate::{
    dtos::{MyceliumProfileData, ToolOperation},
    endpoints::shared::PaginationParams,
    openapi_processor::{list_operations, ServiceOpenApiSchema},
};

use actix_web::{get, web, HttpResponse, Responder};
use myc_http_tools::{
    utils::HttpJsonResponse,
    wrappers::default_response_to_http_response::handle_mapped_error,
};
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};

// ? ---------------------------------------------------------------------------
// ? Configure application
// ? ---------------------------------------------------------------------------

pub fn configure(config: &mut web::ServiceConfig) {
    config.service(list_operations_url);
}

// ? ---------------------------------------------------------------------------
// ? Define API structs
// ? ---------------------------------------------------------------------------

#[derive(Deserialize, ToSchema, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct ListOperationsParams {
    query: Option<String>,
    method: Option<String>,
    score_cutoff: Option<usize>,
}

// ? ---------------------------------------------------------------------------
// ? Define API paths
// ? ---------------------------------------------------------------------------

/// List routes by service
///
/// This function is restricted to the GatewayManager users. List routes by
/// service name or service id.
///
#[utoipa::path(
    get,
    params(
        ListOperationsParams,
    ),
    responses(
        (
            status = 500,
            description = "Unknown internal server error.",
            body = HttpJsonResponse,
        ),
        (
            status = 403,
            description = "Forbidden.",
            body = HttpJsonResponse,
        ),
        (
            status = 401,
            description = "Unauthorized.",
            body = HttpJsonResponse,
        ),
        (
            status = 204,
            description = "Not found.",
        ),
        (
            status = 200,
            description = "Fetching success.",
            body = [ToolOperation],
        ),
    ),
)]
#[get("")]
pub async fn list_operations_url(
    query: web::Query<ListOperationsParams>,
    page: web::Query<PaginationParams>,
    profile: MyceliumProfileData,
    app_module: web::Data<ServiceOpenApiSchema>,
) -> impl Responder {
    match list_operations(
        profile.to_profile(),
        query.query.to_owned(),
        query.method.to_owned(),
        query.score_cutoff.to_owned(),
        page.page_size.map(|s| s as usize),
        page.skip.map(|s| s as usize),
        app_module.clone(),
    )
    .await
    {
        Ok(res) => match res.count {
            0 => HttpResponse::NoContent().finish(),
            _ => HttpResponse::Ok().json(res),
        },
        Err(err) => handle_mapped_error(err),
    }
}
