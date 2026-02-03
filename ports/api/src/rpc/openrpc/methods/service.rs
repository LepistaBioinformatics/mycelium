use super::super::schema;
use crate::rpc::params;

pub fn methods() -> Vec<serde_json::Value> {
    let list_discoverable_services_schema =
        schema::param_schema_value::<params::ListDiscoverableServicesParams>();

    vec![serde_json::json!({
        "name": "service.listDiscoverableServices",
        "summary": "List discoverable services",
        "description": "Lists public discoverable services (tools and contexts). Optional filters: id, name. Uses MemDb.",
        "tags": [{ "name": "service" }],
        "params": [{ "name": "params", "required": false, "schema": list_discoverable_services_schema }],
        "result": { "name": "result", "description": "Object with description, tools, contexts, lastUpdated or null if not found", "schema": { "type": "object" } },
        "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
    })]
}
