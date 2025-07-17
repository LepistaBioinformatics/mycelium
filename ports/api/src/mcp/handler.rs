use crate::openapi_processor::ServiceOpenApiSchema;

use async_trait::async_trait;
use futures::Future;
use mycelium_base::utils::errors::{execution_err, MappedErrors};
use mycelium_openapi::Operation;
use reqwest::Client;
use rust_mcp_sdk::{
    macros::{mcp_tool, JsonSchema},
    mcp_server::ServerHandler,
    schema::{
        schema_utils::CallToolError, CallToolRequest, CallToolResult,
        ListToolsRequest, ListToolsResult, RpcError,
    },
    McpServer,
};
//use rmcp::{
//    handler::server::tool::ToolRouter,
//    model::{Annotated, CallToolResult, RawContent},
//    tool, tool_router,
//};
use serde::{self, Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashMap, sync::Arc};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InputSchema {
    r#type: String,
    properties: HashMap<String, Value>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ToolDef {
    name: String,
    method: String,
    path: String,
    title: String,
    description: String,
    input_schema: InputSchema,
}

impl ToolDef {
    pub fn from_openapi_operation(
        path: &String,
        method: &String,
        operation: &Operation,
    ) -> Result<Self, MappedErrors> {
        let name = operation
            .operation_id
            .clone()
            .ok_or(execution_err("Operation ID is required"))?;

        let title = operation
            .summary
            .clone()
            .ok_or(execution_err("Summary is required"))?;

        let operation_description = operation
            .description
            .clone()
            .ok_or(execution_err("Description is required"))?;

        let description =
            format!("HTTP {} {}\n\n{}", method, path, operation_description);

        let properties = operation
            .parameters
            .clone()
            .unwrap_or_default()
            .into_iter()
            .map(|parameter| (parameter.name, serde_json::json!({}))) // parameter.schema
            .collect::<HashMap<String, Value>>();

        let input_schema = InputSchema {
            r#type: "object".to_string(),
            properties,
        };

        Ok(Self {
            name,
            method: method.clone(),
            path: path.clone(),
            title,
            description,
            input_schema,
        })
    }
}

#[mcp_tool(
    name = "say_hello_world",
    description = "Prints \"Hello World!\" message"
)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct SayHelloTool {}

#[derive(Clone)]
pub(crate) struct MyceliumMcpHandler {
    tools: Arc<Vec<ToolDef>>,
}

#[async_trait]
impl ServerHandler for MyceliumMcpHandler {
    async fn handle_list_tools_request(
        &self,
        request: ListToolsRequest,
        runtime: &dyn McpServer,
    ) -> Result<ListToolsResult, RpcError> {
        Ok(ListToolsResult {
            tools: vec![],
            meta: None,
            next_cursor: None,
        })
    }

    async fn handle_call_tool_request(
        &self,
        request: CallToolRequest,
        runtime: &dyn McpServer,
    ) -> Result<CallToolResult, CallToolError> {
        unimplemented!()
    }
}
