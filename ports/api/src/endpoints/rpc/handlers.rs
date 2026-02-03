use super::dispatchers::{
    dispatch_account_manager, dispatch_beginners, dispatch_gateway_manager,
    dispatch_guest_manager, dispatch_managers,
};
use super::openrpc;
use super::types::{self, JsonRpcRequest, JsonRpcResponse};
use crate::dtos::MyceliumProfileData;
use crate::openapi_processor::ServiceOpenApiSchema;

use actix_web::{post, web, HttpRequest, HttpResponse, Responder};
use myc_core::models::AccountLifeCycle;
use myc_diesel::repositories::SqlAppModule;
use myc_mem_db::repositories::MemDbAppModule;
use tracing::error;

async fn process_single_request(
    profile: &MyceliumProfileData,
    app_module: &web::Data<SqlAppModule>,
    openrpc_config: &web::Data<openrpc::OpenRpcSpecConfig>,
    req: Option<&HttpRequest>,
    life_cycle_settings: Option<&web::Data<AccountLifeCycle>>,
    mem_module: Option<&web::Data<MemDbAppModule>>,
    tools_schema: Option<&web::Data<ServiceOpenApiSchema>>,
    request: JsonRpcRequest,
) -> JsonRpcResponse {
    let id = request.id.clone();

    if request.jsonrpc.as_deref() != Some(types::JSONRPC_VERSION) {
        return types::error_response(
            id,
            types::JsonRpcError {
                code: types::codes::INVALID_REQUEST,
                message: "jsonrpc must be \"2.0\"".to_string(),
                data: None,
            },
        );
    }

    if request.method == "rpc.discover" {
        let spec = openrpc::generate_openrpc_spec(openrpc_config.get_ref());
        return types::success_response(id, spec);
    }

    let scope = request.method.split('.').next();

    let result = match scope {
        Some("beginners") => {
            dispatch_beginners(
                profile,
                app_module,
                req,
                life_cycle_settings,
                &request.method,
                request.params.clone(),
            )
            .await
        }
        Some("managers") => {
            dispatch_managers(
                profile,
                app_module,
                &request.method,
                request.params.clone(),
            )
            .await
        }
        Some("accountManager") => {
            dispatch_account_manager(
                profile,
                app_module,
                life_cycle_settings,
                &request.method,
                request.params.clone(),
            )
            .await
        }
        Some("gatewayManager") => match (mem_module, tools_schema) {
            (Some(mem), Some(tools)) => {
                dispatch_gateway_manager(
                    profile,
                    mem,
                    tools,
                    &request.method,
                    request.params.clone(),
                )
                .await
            }
            _ => Err(types::JsonRpcError {
                code: types::codes::INTERNAL_ERROR,
                message: "MemDb or Tools schema not available".to_string(),
                data: None,
            }),
        },
        Some("guestManager") => {
            dispatch_guest_manager(
                profile,
                app_module,
                &request.method,
                request.params.clone(),
            )
            .await
        }
        _ => Err(types::JsonRpcError {
            code: types::codes::METHOD_NOT_FOUND,
            message: format!("Method not found: {}", request.method),
            data: None,
        }),
    };

    match result {
        Ok(value) => types::success_response(id, value),
        Err(err) => types::error_response(id, err),
    }
}

#[post("")]
pub async fn admin_jsonrpc_post(
    req: HttpRequest,
    body: web::Bytes,
    profile: MyceliumProfileData,
    app_module: web::Data<SqlAppModule>,
    openrpc_config: web::Data<openrpc::OpenRpcSpecConfig>,
    life_cycle_settings: web::Data<AccountLifeCycle>,
    mem_module: web::Data<MemDbAppModule>,
    tools_schema: web::Data<ServiceOpenApiSchema>,
) -> impl Responder {
    let value: serde_json::Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => {
            error!("JSON-RPC parse error: {}", e);
            let response = types::error_response(
                None,
                types::JsonRpcError {
                    code: types::codes::INVALID_REQUEST,
                    message: format!("Invalid JSON: {}", e),
                    data: None,
                },
            );
            return HttpResponse::Ok().json(response);
        }
    };

    if value.is_object() {
        let request: JsonRpcRequest = match serde_json::from_value(value) {
            Ok(r) => r,
            Err(e) => {
                let response = types::error_response(
                    None,
                    types::JsonRpcError {
                        code: types::codes::INVALID_REQUEST,
                        message: e.to_string(),
                        data: None,
                    },
                );
                return HttpResponse::Ok().json(response);
            }
        };
        let response = process_single_request(
            &profile,
            &app_module,
            &openrpc_config,
            Some(&req),
            Some(&life_cycle_settings),
            Some(&mem_module),
            Some(&tools_schema),
            request,
        )
        .await;
        return HttpResponse::Ok().json(response);
    }

    if value.is_array() {
        let arr = value.as_array().unwrap();
        if arr.is_empty() {
            let response = types::error_response(
                None,
                types::JsonRpcError {
                    code: types::codes::INVALID_REQUEST,
                    message: "Batch array cannot be empty".to_string(),
                    data: None,
                },
            );
            return HttpResponse::Ok().json(response);
        }

        let mut responses = Vec::with_capacity(arr.len());
        for item in arr {
            let request: JsonRpcRequest =
                match serde_json::from_value(item.clone()) {
                    Ok(r) => r,
                    Err(e) => {
                        responses.push(types::error_response(
                            item.get("id").cloned(),
                            types::JsonRpcError {
                                code: types::codes::INVALID_REQUEST,
                                message: e.to_string(),
                                data: None,
                            },
                        ));
                        continue;
                    }
                };
            if request.id.is_none() {
                let _ = process_single_request(
                    &profile,
                    &app_module,
                    &openrpc_config,
                    Some(&req),
                    Some(&life_cycle_settings),
                    Some(&mem_module),
                    Some(&tools_schema),
                    request,
                )
                .await;
                continue;
            }
            let response = process_single_request(
                &profile,
                &app_module,
                &openrpc_config,
                Some(&req),
                Some(&life_cycle_settings),
                Some(&mem_module),
                Some(&tools_schema),
                request,
            )
            .await;
            responses.push(response);
        }
        return HttpResponse::Ok().json(responses);
    }

    let response = types::error_response(
        None,
        types::JsonRpcError {
            code: types::codes::INVALID_REQUEST,
            message: "Request must be an object or non-empty array".to_string(),
            data: None,
        },
    );
    HttpResponse::Ok().json(response)
}
