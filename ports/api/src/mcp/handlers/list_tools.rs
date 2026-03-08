use crate::{
    dtos::ToolOperation,
    mcp::dtos::{McpTool},
    openapi_processor::ServiceOpenApiSchema,
    rpc::types::{success_response, JsonRpcRequest, JsonRpcResponse},
};

use mycelium_openapi::OpenApiSchema;
use serde_json::{json, Value};
use slugify::slugify;
use std::collections::HashMap;

/// Convert a snake_case string to camelCase.
fn snake_to_camel(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut capitalize_next = false;

    for ch in s.chars() {
        if ch == '_' || ch == '-' {
            capitalize_next = true;
        } else if capitalize_next {
            result.extend(ch.to_uppercase());
            capitalize_next = false;
        } else {
            result.push(ch);
        }
    }

    result
}

/// Build a deterministic MCP tool name.
///
/// Format: `{service_name}.{operationIdCamelCase}`
///
/// The operation_id is converted from snake_case to camelCase to match
/// the JSON-RPC method naming convention used in the RPC layer.
///
pub(crate) fn build_mcp_tool_name(
    _method: &str,
    operation_id: Option<&String>,
    service_name: &str,
    path: &str,
) -> String {
    let slug = match operation_id {
        Some(id) => slugify!(id.as_str()),
        None => slugify!(path),
    }
    .replace('-', "_");

    let op_id = snake_to_camel(&slug);

    format!("{}.{}", service_name, op_id)
}

pub(crate) fn handle_list_tools(
    req: &JsonRpcRequest,
    registry: &ServiceOpenApiSchema,
) -> JsonRpcResponse {
    let tools: Vec<McpTool> = registry
        .operations
        .iter()
        .map(|op| tool_operation_to_mcp_tool(op, &registry.docs))
        .collect();

    success_response(req.id.clone(), json!({ "tools": tools }))
}

fn tool_operation_to_mcp_tool(
    op: &ToolOperation,
    docs: &HashMap<String, OpenApiSchema>,
) -> McpTool {
    let name = build_mcp_tool_name(
        &op.method.to_string(),
        op.operation.operation_id.as_ref(),
        &op.service.name,
        &op.path,
    );

    let summary = op.operation.summary.as_deref().unwrap_or("No description");
    let security = format!("{:?}", op.security_group);
    let description =
        format!("{}: {} [{}]", op.service.name, summary, security);

    let input_schema = build_input_schema(op, docs);

    McpTool {
        name,
        description: Some(description),
        input_schema,
    }
}

fn build_input_schema(
    op: &ToolOperation,
    docs: &HashMap<String, OpenApiSchema>,
) -> Value {
    // Attempt full $ref resolution via the OpenAPI doc
    let resolved_op =
        op.operation.operation_id.as_deref().and_then(|op_id| {
            docs.get(&op.service.name)?
                .resolve_input_refs_from_operation_id(op_id)
                .ok()
        });

    let mut path_props = serde_json::Map::new();
    let mut query_props = serde_json::Map::new();
    let mut path_required: Vec<String> = Vec::new();
    let mut query_required: Vec<String> = Vec::new();
    let mut body_schema: Option<Value> = None;
    let mut top_required: Vec<String> = Vec::new();

    if let Some(ref resolved) = resolved_op {
        // ── Parameters ──────────────────────────────────────────────────────
        if let Some(params) =
            resolved.get("parameters").and_then(|p| p.as_array())
        {
            for param in params {
                let name = param
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("unknown")
                    .to_string();

                let location =
                    param.get("in").and_then(|i| i.as_str()).unwrap_or("");

                let required = param
                    .get("required")
                    .and_then(|r| r.as_bool())
                    .unwrap_or(false);

                let mut prop = param
                    .get("schema")
                    .cloned()
                    .unwrap_or(json!({ "type": "string" }));

                if let Some(desc) = param.get("description") {
                    if let Some(obj) = prop.as_object_mut() {
                        obj.insert("description".to_string(), desc.clone());
                    }
                }

                match location {
                    "path" => {
                        if required {
                            path_required.push(name.clone());
                        }
                        path_props.insert(name, prop);
                    }
                    "query" => {
                        if required {
                            query_required.push(name.clone());
                        }
                        query_props.insert(name, prop);
                    }
                    _ => {}
                }
            }
        }

        // ── Request body ─────────────────────────────────────────────────────
        if let Some(rb) = resolved.get("requestBody") {
            let required =
                rb.get("required").and_then(|r| r.as_bool()).unwrap_or(false);

            if let Some(schema) = rb
                .get("content")
                .and_then(|c| c.get("application/json"))
                .and_then(|j| j.get("schema"))
                .cloned()
            {
                body_schema = Some(schema);
                if required {
                    top_required.push("__body".to_string());
                }
            }
        }
    } else {
        // Fallback: use raw (possibly unresolved) parameters.
        // Location is not re-exported from mycelium_openapi so we
        // serialise it to a string for comparison.
        if let Some(params) = &op.operation.parameters {
            for param in params {
                let name = param.name.clone();
                let required = param.required.unwrap_or(false);

                let prop = serde_json::to_value(&param.schema)
                    .unwrap_or(json!({ "type": "string" }));

                let in_str = serde_json::to_value(&param.r#in)
                    .ok()
                    .and_then(|v| v.as_str().map(|s| s.to_string()))
                    .unwrap_or_default();

                match in_str.as_str() {
                    "path" => {
                        if required {
                            path_required.push(name.clone());
                        }
                        path_props.insert(name, prop);
                    }
                    "query" => {
                        if required {
                            query_required.push(name.clone());
                        }
                        query_props.insert(name, prop);
                    }
                    _ => {}
                }
            }
        }
    }

    // ── Assemble top-level schema ────────────────────────────────────────────
    let mut properties = serde_json::Map::new();

    if !path_props.is_empty() {
        let mut path_schema = json!({
            "type": "object",
            "properties": Value::Object(path_props)
        });
        if !path_required.is_empty() {
            path_schema["required"] = json!(path_required);
        }
        properties.insert("__path_params".to_string(), path_schema);
        top_required.push("__path_params".to_string());
    }

    if !query_props.is_empty() {
        let mut query_schema = json!({
            "type": "object",
            "properties": Value::Object(query_props)
        });
        if !query_required.is_empty() {
            query_schema["required"] = json!(query_required);
        }
        properties.insert("__query_params".to_string(), query_schema);
    }

    if let Some(body) = body_schema {
        properties.insert("__body".to_string(), body);
    }

    json!({
        "type": "object",
        "properties": Value::Object(properties),
        "required": top_required
    })
}
