use super::super::errors::{invalid_params, mapped_errors_to_jsonrpc_error};
use super::super::params::{
    ListOperationsParams, ListRoutesParams, ListServicesParams,
};
use super::super::response_kind::fetch_many_response_kind_to_result;
use super::super::types::{self, JsonRpcError};
use crate::dtos::MyceliumProfileData;
use crate::openapi_processor::list_operations;

use actix_web::web;
use myc_core::use_cases::role_scoped::gateway_manager::{
    route::list_routes, service::list_services,
};
use myc_mem_db::repositories::MemDbAppModule;
use shaku::HasComponent;

pub async fn dispatch_gateway_manager(
    profile: &MyceliumProfileData,
    mem_module: &web::Data<MemDbAppModule>,
    tools_schema: &web::Data<crate::openapi_processor::ServiceOpenApiSchema>,
    method: &str,
    params: Option<serde_json::Value>,
) -> Result<serde_json::Value, JsonRpcError> {
    match method {
        "gatewayManager.routes.listRoutes" => {
            let p: ListRoutesParams = params
                .map(serde_json::from_value)
                .transpose()
                .map_err(|e| invalid_params(e.to_string()))?
                .unwrap_or_default();
            let result = list_routes(
                profile.to_profile(),
                p.id,
                p.name,
                p.page_size,
                p.skip,
                Box::new(&*mem_module.resolve_ref()),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;
            fetch_many_response_kind_to_result(result)
        }
        "gatewayManager.services.listServices" => {
            let p: ListServicesParams = params
                .map(serde_json::from_value)
                .transpose()
                .map_err(|e| invalid_params(e.to_string()))?
                .unwrap_or_default();
            let result = list_services(
                profile.to_profile(),
                p.id,
                p.name,
                p.page_size,
                p.skip,
                Box::new(&*mem_module.resolve_ref()),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;
            fetch_many_response_kind_to_result(result)
        }
        "gatewayManager.tools.listOperations" => {
            let p: ListOperationsParams = params
                .map(serde_json::from_value)
                .transpose()
                .map_err(|e| invalid_params(e.to_string()))?
                .unwrap_or_default();
            let result = list_operations(
                profile.to_profile(),
                p.query,
                p.method,
                p.score_cutoff,
                p.page_size,
                p.skip,
                tools_schema.clone(),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;
            serde_json::to_value(result).map_err(|e| JsonRpcError {
                code: types::codes::INTERNAL_ERROR,
                message: e.to_string(),
                data: None,
            })
        }
        _ => Err(JsonRpcError {
            code: types::codes::METHOD_NOT_FOUND,
            message: format!("Method not found: {}", method),
            data: None,
        }),
    }
}
