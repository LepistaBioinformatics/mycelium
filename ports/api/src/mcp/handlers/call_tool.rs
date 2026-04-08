use crate::{
    mcp::{
        dtos::{CallToolParams, CallToolResult, ToolContent},
        handlers::list_tools::build_mcp_tool_name,
    },
    models::api_config::ApiConfig,
    openapi_processor::ServiceOpenApiSchema,
    rpc::types::{
        self, error_response, success_response, JsonRpcRequest, JsonRpcResponse,
    },
};

use actix_web::{web, HttpRequest};
use awc::Client;
use serde_json::Value;
use std::time::Duration;
use url::form_urlencoded;

pub(crate) async fn handle_call_tool(
    http_req: &HttpRequest,
    rpc_req: &JsonRpcRequest,
    registry: &web::Data<ServiceOpenApiSchema>,
    client: &web::Data<Client>,
    config: &web::Data<ApiConfig>,
) -> JsonRpcResponse {
    // ── Parse params ─────────────────────────────────────────────────────────
    let call: CallToolParams = match rpc_req
        .params
        .as_ref()
        .and_then(|p| serde_json::from_value(p.clone()).ok())
    {
        Some(c) => c,
        None => {
            return error_response(
                rpc_req.id.clone(),
                types::JsonRpcError {
                    code: types::codes::INVALID_PARAMS,
                    message: "Missing or invalid params for tools/call"
                        .to_string(),
                    data: None,
                },
            );
        }
    };

    // ── Find ToolOperation by deterministic name ─────────────────────────────
    let tool_op = registry.operations.iter().find(|op| {
        build_mcp_tool_name(
            &op.method.to_string(),
            op.operation.operation_id.as_ref(),
            &op.service.name,
            &op.path,
        ) == call.name
    });

    let tool_op = match tool_op {
        Some(op) => op,
        None => {
            return error_response(
                rpc_req.id.clone(),
                types::JsonRpcError {
                    code: types::codes::INVALID_PARAMS,
                    message: format!("Tool not found: {}", call.name),
                    data: None,
                },
            );
        }
    };

    // ── Extract structured arguments ─────────────────────────────────────────
    let args = call
        .arguments
        .unwrap_or(Value::Object(serde_json::Map::new()));

    let path_params = args
        .get("__path_params")
        .cloned()
        .unwrap_or(Value::Object(serde_json::Map::new()));
    let query_params = args.get("__query_params").cloned();
    let body = args.get("__body").cloned();

    // ── Substitute path parameters ───────────────────────────────────────────
    let mut path = tool_op.path.clone();
    if let Some(obj) = path_params.as_object() {
        for (key, value) in obj {
            let placeholder = format!("{{{key}}}");
            let val_str = match value {
                Value::String(s) => s.clone(),
                other => other.to_string().trim_matches('"').to_string(),
            };
            path = path.replace(&placeholder, &val_str);
        }
    }

    // ── Build loopback URL ───────────────────────────────────────────────────
    let ip = &config.service_ip;
    let port = config.service_port;
    let path = if path.starts_with('/') {
        path
    } else {
        format!("/{path}")
    };
    let mut url = format!("http://{ip}:{port}{path}");

    tracing::info!(mcp.loopback_url = %url, "MCP loopback URL");

    if let Some(qp) = &query_params {
        if let Some(obj) = qp.as_object() {
            let qs: String = form_urlencoded::Serializer::new(String::new())
                .extend_pairs(obj.iter().map(|(k, v)| {
                    let val = match v {
                        Value::String(s) => s.clone(),
                        other => {
                            other.to_string().trim_matches('"').to_string()
                        }
                    };
                    (k.as_str(), val)
                }))
                .finish();

            if !qs.is_empty() {
                url = format!("{url}?{qs}");
            }
        }
    }

    // ── Build downstream request ─────────────────────────────────────────────
    let method_str = tool_op.method.to_string().to_uppercase();
    let mut downstream = match method_str.as_str() {
        "GET" => client.get(&url),
        "POST" => client.post(&url),
        "PUT" => client.put(&url),
        "DELETE" => client.delete(&url),
        "PATCH" => client.patch(&url),
        "HEAD" => client.head(&url),
        "OPTIONS" => client.options(&url),
        _ => client.get(&url),
    };

    downstream =
        downstream.timeout(Duration::from_secs(config.gateway_timeout));

    // Forward Authorization header from the original MCP request
    if let Some(auth) = http_req.headers().get("Authorization") {
        if let Ok(auth_str) = auth.to_str() {
            tracing::debug!(mcp.auth_header = %auth_str, "Forwarding Authorization header");
            downstream = downstream.insert_header(("Authorization", auth_str));
        }
    }

    // Forward Connection String header from the original MCP request
    if let Some(cs) = http_req.headers().get("x-mycelium-connection-string") {
        if let Ok(cs_str) = cs.to_str() {
            tracing::debug!(mcp.cs_header = %cs_str, "Forwarding Connection String header");
            downstream = downstream
                .insert_header(("x-mycelium-connection-string", cs_str));
        }
    }

    // ── Send ─────────────────────────────────────────────────────────────────
    let send_result = if let Some(body_val) = body {
        match serde_json::to_vec(&body_val) {
            Ok(bytes) => {
                downstream
                    .insert_header(("Content-Type", "application/json"))
                    .send_body(bytes)
                    .await
            }
            Err(e) => {
                return error_response(
                    rpc_req.id.clone(),
                    types::JsonRpcError {
                        code: types::codes::INTERNAL_ERROR,
                        message: format!(
                            "Failed to serialize request body: {e}"
                        ),
                        data: None,
                    },
                );
            }
        }
    } else {
        downstream.send().await
    };

    // ── Build MCP result from downstream response ────────────────────────────
    let tool_result = match send_result {
        Ok(mut resp) => {
            let status = resp.status().as_u16();
            let body_text = resp
                .body()
                .limit(10 * 1024 * 1024)
                .await
                .map(|b| String::from_utf8_lossy(&b).into_owned())
                .unwrap_or_else(|e| {
                    format!("Failed to read response body: {e}")
                });

            let is_error = if status >= 400 { Some(true) } else { None };

            CallToolResult {
                content: vec![ToolContent {
                    r#type: "text".to_string(),
                    text: if status >= 400 {
                        format!("HTTP {status}\n{body_text}")
                    } else {
                        body_text
                    },
                }],
                is_error,
            }
        }
        Err(e) => CallToolResult {
            content: vec![ToolContent {
                r#type: "text".to_string(),
                text: format!("Loopback request failed: {e}"),
            }],
            is_error: Some(true),
        },
    };

    match serde_json::to_value(tool_result) {
        Ok(v) => success_response(rpc_req.id.clone(), v),
        Err(e) => error_response(
            rpc_req.id.clone(),
            types::JsonRpcError {
                code: types::codes::INTERNAL_ERROR,
                message: format!("Failed to serialize tool result: {e}"),
                data: None,
            },
        ),
    }
}
