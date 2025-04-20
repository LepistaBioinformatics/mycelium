use crate::dtos::Tool;

use actix_web::{get, web, HttpRequest, HttpResponse, Responder};
use myc_core::{
    domain::dtos::service::Service,
    use_cases::service::service::list_discoverable_services,
};
use myc_http_tools::{
    utils::HttpJsonResponse,
    wrappers::default_response_to_http_response::handle_mapped_error,
};
use myc_mem_db::repositories::MemDbAppModule;
use mycelium_base::entities::FetchManyResponseKind;
use serde::Deserialize;
use serde_json::json;
use shaku::HasComponent;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

// ? ---------------------------------------------------------------------------
// ? Configure application
// ? ---------------------------------------------------------------------------

pub fn configure(config: &mut web::ServiceConfig) {
    config.service(list_discoverable_services_url);
}

// ? ---------------------------------------------------------------------------
// ? Define API structs
// ? ---------------------------------------------------------------------------

#[derive(Deserialize, ToSchema, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct ListServicesParams {
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
        ListServicesParams,
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
            body = [Service],
        ),
    ),
)]
#[get("")]
pub async fn list_discoverable_services_url(
    query: web::Query<ListServicesParams>,
    request: HttpRequest,
    app_module: web::Data<MemDbAppModule>,
) -> impl Responder {
    match list_discoverable_services(
        query.id.to_owned(),
        query.name.to_owned(),
        Box::new(&*app_module.resolve_ref()),
    )
    .await
    {
        Ok(res) => match res {
            FetchManyResponseKind::Found(services) => {
                let tools = services
                    .into_iter()
                    .map(|service| {
                        match Tool::from_service(service, request.full_url()) {
                            Ok(tool) => Some(tool),
                            Err(err) => {
                                tracing::error!(
                                    "Error converting service to tool: {err}"
                                );

                                None
                            }
                        }
                    })
                    .filter_map(|tool| tool)
                    .collect::<Vec<_>>();

                HttpResponse::Ok().json(tools)
            }
            FetchManyResponseKind::FoundPaginated {
                count,
                skip,
                size,
                records,
            } => HttpResponse::Ok().json(json!({
                "count": count,
                "skip": skip,
                "size": size,
                "records": records,
            })),
            _ => HttpResponse::NoContent().finish(),
        },
        Err(err) => handle_mapped_error(err),
    }
}
