use super::super::schema;
use crate::rpc::{method_names, params};

pub fn methods() -> Vec<serde_json::Value> {
    let fetch_resource_audit_trail_schema =
        schema::param_schema_value::<params::FetchResourceAuditTrailParams>();

    vec![serde_json::json!({
        "name": method_names::AUDIT_RESOURCE_TRAIL_FETCH,
        "summary": "Fetch the audit trail of a resource",
        "description": "Returns the immutable audit trail for a single resource, newest first. Visible to staff (any resource), tenant owners/managers (resources of their own tenant), and personal-account owners (their own account). Not role scoped -- permission branching happens inside the use case.",
        "tags": [{ "name": "audit" }, { "name": "resourceTrail" }],
        "params": [{ "name": "params", "required": true, "schema": fetch_resource_audit_trail_schema }],
        "result": { "name": "result", "description": "Audit trail (FetchManyResponseKind), newest first", "schema": { "type": "array", "items": { "type": "object" } } },
        "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
    })]
}
