use crate::rpc::types::{success_response, JsonRpcRequest, JsonRpcResponse};
use serde_json::json;

pub(crate) fn handle_initialize(req: &JsonRpcRequest) -> JsonRpcResponse {
    success_response(
        req.id.clone(),
        json!({
            "protocolVersion": "2025-03-26",
            "capabilities": {
                "tools": {}
            },
            "serverInfo": {
                "name": "mycelium",
                "version": env!("CARGO_PKG_VERSION")
            }
        }),
    )
}
