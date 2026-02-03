use super::super::{
    errors::{invalid_params, mapped_errors_to_jsonrpc_error},
    method_names,
    params::ListDiscoverableServicesParams,
    types::{self, JsonRpcError},
};
use crate::dtos::Tool;

use actix_web::web;
use chrono::{DateTime, Local};
use myc_core::domain::dtos::health_check_info::HealthStatus;
use myc_core::use_cases::service::service::list_discoverable_services;
use myc_mem_db::repositories::MemDbAppModule;
use mycelium_base::entities::FetchManyResponseKind;
use serde::Serialize;
use shaku::HasComponent;

fn description() -> String {
    "Describe public services, including the context where the service should run".to_string()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ListServicesResponse {
    description: String,
    contexts: Vec<Tool>,
    last_updated: Option<DateTime<Local>>,
    tools: Vec<Tool>,
}

pub async fn dispatch_service(
    _profile: &crate::dtos::MyceliumProfileData,
    mem_module: &web::Data<MemDbAppModule>,
    method: &str,
    params: Option<serde_json::Value>,
) -> Result<serde_json::Value, JsonRpcError> {
    match method {
        method_names::SERVICE_LIST_DISCOVERABLE_SERVICES => {
            let p: ListDiscoverableServicesParams = params
                .map(serde_json::from_value)
                .transpose()
                .map_err(|e| invalid_params(e.to_string()))?
                .unwrap_or_default();
            let result = list_discoverable_services(
                p.id,
                p.name,
                Box::new(&*mem_module.resolve_ref()),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;
            match result {
                FetchManyResponseKind::Found(services) => {
                    let tools: Vec<Tool> = services
                        .into_iter()
                        .filter_map(|service| {
                            Tool::from_service(service).map_err(|err| {
                                tracing::error!("Error converting service to tool: {err}");
                                err
                            })
                            .ok()
                        })
                        .collect();
                    let last_updated = tools
                        .iter()
                        .filter_map(|tool| match &tool.health_status {
                            HealthStatus::Healthy { checked_at } => {
                                Some(*checked_at)
                            }
                            HealthStatus::Unhealthy { checked_at, .. } => {
                                Some(*checked_at)
                            }
                            HealthStatus::Unavailable {
                                checked_at, ..
                            } => Some(*checked_at),
                            _ => None,
                        })
                        .max();
                    let contexts: Vec<Tool> = tools
                        .iter()
                        .filter(|t| t.is_context_api)
                        .cloned()
                        .collect();
                    let tools_only: Vec<Tool> = tools
                        .into_iter()
                        .filter(|t| !t.is_context_api)
                        .collect();
                    let response = ListServicesResponse {
                        description: description(),
                        tools: tools_only,
                        last_updated,
                        contexts,
                    };
                    serde_json::to_value(response).map_err(|e| JsonRpcError {
                        code: types::codes::INTERNAL_ERROR,
                        message: e.to_string(),
                        data: None,
                    })
                }
                FetchManyResponseKind::FoundPaginated { .. } => {
                    Err(JsonRpcError {
                        code: types::codes::INTERNAL_ERROR,
                        message:
                            "Pagination is not supported for this endpoint"
                                .to_string(),
                        data: None,
                    })
                }
                FetchManyResponseKind::NotFound => Ok(serde_json::Value::Null),
            }
        }
        _ => Err(JsonRpcError {
            code: types::codes::METHOD_NOT_FOUND,
            message: format!("Method not found: {}", method),
            data: None,
        }),
    }
}
