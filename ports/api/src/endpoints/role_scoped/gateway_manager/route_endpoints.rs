use crate::{dtos::MyceliumProfileData, endpoints::shared::PaginationParams};

use actix_web::{get, web, Responder};
use myc_core::{
    domain::dtos::route::Route,
    use_cases::role_scoped::gateway_manager::route::list_routes,
};
use myc_http_tools::{
    utils::HttpJsonResponse,
    wrappers::default_response_to_http_response::{
        fetch_many_response_kind, handle_mapped_error,
    },
};
use myc_mem_db::repositories::MemDbAppModule;
use serde::Deserialize;
use shaku::HasComponent;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

// ? ---------------------------------------------------------------------------
// ? Configure application
// ? ---------------------------------------------------------------------------

pub fn configure(config: &mut web::ServiceConfig) {
    config.service(list_routes_url);
}

// ? ---------------------------------------------------------------------------
// ? Define API structs
// ? ---------------------------------------------------------------------------

#[derive(Deserialize, ToSchema, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct ListRoutesByServiceParams {
    id: Option<Uuid>,
    name: Option<String>,
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
        ListRoutesByServiceParams,
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
            body = [Route],
        ),
    ),
)]
#[get("")]
pub async fn list_routes_url(
    query: web::Query<ListRoutesByServiceParams>,
    page: web::Query<PaginationParams>,
    profile: MyceliumProfileData,
    app_module: web::Data<MemDbAppModule>,
) -> impl Responder {
    match list_routes(
        profile.to_profile(),
        query.id.to_owned(),
        query.name.to_owned(),
        page.page_size,
        page.skip,
        Box::new(&*app_module.resolve_ref()),
    )
    .await
    {
        Ok(res) => fetch_many_response_kind(res),
        Err(err) => handle_mapped_error(err),
    }
}
