use crate::{dtos::Tool, settings::GATEWAY_API_SCOPE};

use actix_web::{get, web, HttpResponse, Responder};
use chrono::{DateTime, Local};
use myc_core::{
    domain::dtos::health_check_info::HealthStatus,
    use_cases::service::service::list_discoverable_services,
};
use myc_http_tools::{
    utils::HttpJsonResponse,
    wrappers::default_response_to_http_response::handle_mapped_error,
};
use myc_mem_db::repositories::MemDbAppModule;
use mycelium_base::entities::FetchManyResponseKind;
use serde::{Deserialize, Serialize};
use shaku::HasComponent;
use utoipa::{IntoParams, ToResponse, ToSchema};
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

#[derive(Serialize, ToSchema, ToResponse)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ListServicesResponse {
    /// Description
    ///
    /// The description of the service.
    ///
    description: String,

    /// The contexts
    ///
    /// The contexts of the service. This key snould include the context where
    /// the service should run, including authentication and authorization
    /// information.
    ///
    contexts: Vec<Tool>,

    /// The last updated date
    ///
    /// The last updated date of the service.
    ///
    last_updated: Option<DateTime<Local>>,

    /// A list of tools
    ///
    /// A list of tools that are discoverable by the service.
    ///
    tools: Vec<Tool>,
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
            body = ListServicesResponse,
        ),
    ),
    security(()),
)]
#[get("")]
pub async fn list_discoverable_services_url(
    query: web::Query<ListServicesParams>,
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
                //
                // Build host from gateway api scope
                //
                let host = String::from(format!("/{GATEWAY_API_SCOPE}"));

                let tools = services
                    .into_iter()
                    .map(|service| {
                        match Tool::from_service(service, host.clone()) {
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

                let last_updated = tools
                    .iter()
                    .map(|tool| match tool.health_status {
                        HealthStatus::Healthy { checked_at } => {
                            Some(checked_at)
                        }
                        HealthStatus::Unhealthy { checked_at, .. } => {
                            Some(checked_at)
                        }
                        HealthStatus::Unavailable { checked_at, .. } => {
                            Some(checked_at)
                        }
                        _ => None,
                    })
                    .max()
                    .unwrap_or_default();

                let contexts = tools
                    .iter()
                    .filter(|tool| tool.is_context_api)
                    .map(|tool| tool.to_owned())
                    .collect::<Vec<_>>();

                let tools = tools
                    .iter()
                    .filter(|tool| !tool.is_context_api)
                    .map(|tool| tool.to_owned())
                    .collect::<Vec<_>>();

                HttpResponse::Ok().json(ListServicesResponse {
                    description: get_description(),
                    tools,
                    last_updated,
                    contexts,
                })
            }
            FetchManyResponseKind::FoundPaginated { .. } => {
                tracing::error!(
                    "Pagination is not supported for this endpoint"
                );

                HttpResponse::BadRequest().json(HttpJsonResponse::new_message(
                    "Unexpected internal error",
                ))
            }
            _ => HttpResponse::NoContent().finish(),
        },
        Err(err) => handle_mapped_error(err),
    }
}

fn get_description() -> String {
    "Describe public services, including the context where the service should run".to_string()
}
