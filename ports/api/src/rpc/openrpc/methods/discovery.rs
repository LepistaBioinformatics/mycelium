use crate::rpc::method_names;

pub fn method() -> serde_json::Value {
    serde_json::json!({
        "name": method_names::RPC_DISCOVER,
        "summary": "Discover the API",
        "description": "Returns this OpenRPC spec document. No params.",
        "tags": [{ "name": "discovery" }],
        "params": [],
        "result": {
            "name": "openrpc",
            "description": "The OpenRPC specification document",
            "schema": { "type": "object" }
        }
    })
}
