use super::{
    dispatchers::{
        dispatch_account_manager, dispatch_beginners, dispatch_gateway_manager,
        dispatch_guest_manager, dispatch_managers, dispatch_service,
        dispatch_staff, dispatch_subscriptions_manager,
        dispatch_system_manager, dispatch_tenant_manager,
        dispatch_tenant_owner, dispatch_users_manager,
    },
    openrpc,
    types::{self, JsonRpcRequest, JsonRpcResponse},
};
use crate::{
    dtos::MyceliumProfileData, openapi_processor::ServiceOpenApiSchema,
};

use actix_web::{post, web, HttpRequest, HttpResponse, Responder};
use myc_core::models::AccountLifeCycle;
use myc_diesel::repositories::SqlAppModule;
use myc_mem_db::repositories::MemDbAppModule;
use tracing::{error, info, info_span};

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
    let method = request.method.as_str();
    let span = info_span!("rpc_call", rpc.method = %method);
    let _guard = span.enter();

    info!(rpc.method = %method, "RPC request");

    let id = request.id.clone();

    if request.jsonrpc.as_deref() != Some(types::JSONRPC_VERSION) {
        info!(rpc.method = %method, success = false, error = "invalid_jsonrpc_version", "RPC response");
        return types::error_response(
            id,
            types::JsonRpcError {
                code: types::codes::INVALID_REQUEST,
                message: "jsonrpc must be \"2.0\"".to_string(),
                data: None,
            },
        );
    }

    if request.method == super::method_names::RPC_DISCOVER {
        let spec = openrpc::generate_openrpc_spec(openrpc_config.get_ref());
        info!(rpc.method = %method, success = true, "RPC response");
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
        Some("subscriptionsManager") => {
            dispatch_subscriptions_manager(
                profile,
                app_module,
                life_cycle_settings,
                &request.method,
                request.params.clone(),
            )
            .await
        }
        Some("systemManager") => {
            dispatch_system_manager(
                profile,
                app_module,
                life_cycle_settings,
                &request.method,
                request.params.clone(),
            )
            .await
        }
        Some("tenantManager") => {
            dispatch_tenant_manager(
                profile,
                app_module,
                life_cycle_settings,
                &request.method,
                request.params.clone(),
            )
            .await
        }
        Some("tenantOwner") => {
            dispatch_tenant_owner(
                profile,
                app_module,
                &request.method,
                request.params.clone(),
            )
            .await
        }
        Some("userManager") => {
            dispatch_users_manager(
                profile,
                app_module,
                &request.method,
                request.params.clone(),
            )
            .await
        }
        Some("service") => match mem_module {
            Some(mem) => {
                dispatch_service(
                    profile,
                    mem,
                    &request.method,
                    request.params.clone(),
                )
                .await
            }
            _ => Err(types::JsonRpcError {
                code: types::codes::INTERNAL_ERROR,
                message: "MemDb not available for service scope".to_string(),
                data: None,
            }),
        },
        Some("staff") => {
            dispatch_staff(
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
        Ok(value) => {
            info!(rpc.method = %method, success = true, "RPC response");
            types::success_response(id, value)
        }
        Err(err) => {
            info!(
                rpc.method = %method,
                success = false,
                error_code = err.code,
                error_message = %err.message,
                "RPC response"
            );
            types::error_response(id, err)
        }
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
    let span = info_span!("adm_rpc", path = "_adm/rpc");
    let _guard = span.enter();

    let value: serde_json::Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => {
            error!(path = "_adm/rpc", "JSON-RPC parse error: {}", e);
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
                info!(path = "_adm/rpc", error = %e, "RPC invalid request body");
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
        info!(
            path = "_adm/rpc",
            batch_size = arr.len(),
            "RPC batch request"
        );
        if arr.is_empty() {
            info!(
                path = "_adm/rpc",
                error = "batch_empty",
                "RPC batch array cannot be empty"
            );
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

    info!(
        path = "_adm/rpc",
        error = "invalid_request_shape",
        "RPC request must be object or non-empty array"
    );
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
