use crate::{
    mcp::handlers::{handle_call_tool, handle_initialize, handle_list_tools},
    models::api_config::ApiConfig,
    openapi_processor::ServiceOpenApiSchema,
    rpc::types::{self, error_response, JsonRpcRequest, JSONRPC_VERSION},
};

use actix_web::{post, web, HttpRequest, HttpResponse, Responder};
use awc::Client;
use tracing::info;

pub fn configure(config: &mut web::ServiceConfig) {
    config.service(mcp_endpoint);
}

#[post("")]
async fn mcp_endpoint(
    http_req: HttpRequest,
    body: web::Bytes,
    tools_registry: web::Data<ServiceOpenApiSchema>,
    client: web::Data<Client>,
    api_config: web::Data<ApiConfig>,
) -> impl Responder {
    // ── Parse JSON body ──────────────────────────────────────────────────────
    let request: JsonRpcRequest = match serde_json::from_slice(&body) {
        Ok(r) => r,
        Err(e) => {
            return HttpResponse::Ok().json(error_response(
                None,
                types::JsonRpcError {
                    code: types::codes::INVALID_REQUEST,
                    message: format!("Invalid JSON: {e}"),
                    data: None,
                },
            ));
        }
    };

    // ── Validate JSON-RPC version ────────────────────────────────────────────
    if request.jsonrpc.as_deref() != Some(JSONRPC_VERSION) {
        return HttpResponse::Ok().json(error_response(
            request.id.clone(),
            types::JsonRpcError {
                code: types::codes::INVALID_REQUEST,
                message: format!(r#"jsonrpc must be "{JSONRPC_VERSION}""#),
                data: None,
            },
        ));
    }

    // ── MCP notifications (no id) ────────────────────────────────────────────
    if request.id.is_none()
        && request.method.starts_with("notifications/")
    {
        return HttpResponse::Ok().finish();
    }

    info!(mcp.method = %request.method, "MCP request");

    // ── Dispatch ─────────────────────────────────────────────────────────────
    let response = match request.method.as_str() {
        "initialize" => handle_initialize(&request),
        "tools/list" => handle_list_tools(&request, &tools_registry),
        "tools/call" => {
            handle_call_tool(
                &http_req,
                &request,
                &tools_registry,
                &client,
                &api_config,
            )
            .await
        }
        _ => error_response(
            request.id.clone(),
            types::JsonRpcError {
                code: types::codes::METHOD_NOT_FOUND,
                message: format!(
                    "Method not found: {}",
                    request.method
                ),
                data: None,
            },
        ),
    };

    HttpResponse::Ok().json(response)
}
