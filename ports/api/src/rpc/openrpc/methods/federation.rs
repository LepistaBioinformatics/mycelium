use super::super::schema;
use crate::rpc::{method_names, params};

fn method(
    name: &str,
    summary: &str,
    description: &str,
    params_schema: Option<serde_json::Value>,
) -> serde_json::Value {
    let params = match params_schema {
        Some(schema) => {
            serde_json::json!([{ "name": "params", "required": true, "schema": schema }])
        }
        None => serde_json::json!([]),
    };

    serde_json::json!({
        "name": name,
        "summary": summary,
        "description": description,
        "tags": [{ "name": "federation" }],
        "params": params,
        "result": { "name": "result", "schema": { "type": "object" } },
        "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
    })
}

pub fn methods() -> Vec<serde_json::Value> {
    let peer =
        || Some(schema::param_schema_value::<params::FederationPeerParams>());

    vec![
        method(
            method_names::FEDERATION_PEERS_LIST,
            "List federation peers",
            "Every declared peer with its status, negotiated assertion kinds, stored key ids, last contact and failure count. Requires Staff privileges.",
            None,
        ),
        method(
            method_names::FEDERATION_PEERS_QUARANTINE,
            "Quarantine a peer",
            "Stops serving the peer from its next request. Requires Staff privileges.",
            peer(),
        ),
        method(
            method_names::FEDERATION_PEERS_RELEASE,
            "Release a peer",
            "Lifts a quarantine; the peer is declared again and re-handshaked on the next health round. Requires Staff privileges.",
            peer(),
        ),
        method(
            method_names::FEDERATION_SET_ENABLED,
            "Switch federation on or off",
            "The runtime switch. Off is the kill-switch: the self-description answers 404 and every peer request is refused. On is refused while [core.federation] is absent. Requires Staff privileges.",
            Some(schema::param_schema_value::<params::SetFederationEnabledParams>()),
        ),
    ]
}
