use super::super::schema;
use crate::rpc::params;

pub fn methods() -> Vec<serde_json::Value> {
    let list_routes_schema =
        schema::param_schema_value::<params::ListRoutesParams>();
    let list_services_schema =
        schema::param_schema_value::<params::ListServicesParams>();
    let list_operations_schema =
        schema::param_schema_value::<params::ListOperationsParams>();

    vec![
        serde_json::json!({
            "name": "gatewayManager.routes.listRoutes",
            "summary": "List routes by service",
            "description": "Lists routes filtered by service ID or name. Restricted to GatewayManager users. Uses in-memory route storage.",
            "tags": [{ "name": "gatewayManager" }, { "name": "routes" }],
            "params": [{ "name": "params", "required": false, "schema": list_routes_schema }],
            "result": { "name": "result", "description": "List of routes (FetchManyResponseKind)", "schema": { "type": "array", "items": { "type": "object" } } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": "gatewayManager.services.listServices",
            "summary": "List services",
            "description": "Lists services filtered by ID or name. Restricted to GatewayManager users. Uses in-memory service storage.",
            "tags": [{ "name": "gatewayManager" }, { "name": "services" }],
            "params": [{ "name": "params", "required": false, "schema": list_services_schema }],
            "result": { "name": "result", "description": "List of services (FetchManyResponseKind)", "schema": { "type": "array", "items": { "type": "object" } } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": "gatewayManager.tools.listOperations",
            "summary": "List operations",
            "description": "Lists tool operations from downstream OpenAPI specs with optional search (query, method, scoreCutoff) and pagination. Restricted to GatewayManager users.",
            "tags": [{ "name": "gatewayManager" }, { "name": "tools" }],
            "params": [{ "name": "params", "required": false, "schema": list_operations_schema }],
            "result": { "name": "result", "description": "SearchOperationResponse (records, count, pageSize, skip)", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
    ]
}
