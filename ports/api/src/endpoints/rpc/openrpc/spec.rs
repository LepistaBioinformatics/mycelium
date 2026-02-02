//! Assembly of the OpenRPC spec document: servers, methods, info.

use super::config::OpenRpcSpecConfig;
use super::methods;

const OPENRPC_VERSION: &str = "1.2.6";
const API_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Build the OpenRPC spec document for the admin JSON-RPC API.
pub fn generate_openrpc_spec(config: &OpenRpcSpecConfig) -> serde_json::Value {
    let mut servers = vec![serde_json::json!({
        "name": "Development",
        "url": config.dev_url,
        "summary": "Local or development server"
    })];

    if let Some(ref url) = config.prod_url {
        servers.push(serde_json::json!({
            "name": "Production",
            "url": url,
            "summary": "Production server"
        }));
    }

    let methods = methods::all_methods();

    serde_json::json!({
        "openrpc": OPENRPC_VERSION,
        "info": {
            "title": "Mycelium Admin JSON-RPC API",
            "description": "JSON-RPC 2.0 API for Mycelium admin operations (managers). Supports single request and batch. Scope and permission checks are enforced by use cases.",
            "version": API_VERSION,
            "contact": {
                "name": "Samuel Galvão Elias",
                "url": "https://github.com/sgelias/mycelium"
            }
        },
        "servers": servers,
        "methods": methods,
        "components": { "schemas": {} }
    })
}
