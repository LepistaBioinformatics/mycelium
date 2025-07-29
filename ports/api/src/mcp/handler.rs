use crate::{
    dtos::ToolOperation,
    openapi_processor::{
        build_operation_id, resolve_refs, ServiceOpenApiSchema,
    },
};

use mycelium_openapi::Operation;
use rmcp::{
    handler::server::ServerHandler,
    model::{
        CallToolRequestMethod, CallToolRequestParam, CallToolResult,
        ListToolsResult, PaginatedRequestParam, Tool,
    },
    service::{RequestContext, RoleServer},
    ErrorData,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map as JsonObject, Value};
use std::future::Future;
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct MCPParameter {
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    schema: Value,
}

impl MCPParameter {
    pub(crate) fn new(description: Option<String>, schema: Value) -> Self {
        Self {
            description,
            schema,
        }
    }
}

impl From<ToolOperation> for Tool {
    fn from(tool_operation: ToolOperation) -> Self {
        let default_parameter_vec = vec![];

        let operation_id =
            tool_operation.operation.operation_id.as_ref().unwrap();

        let mut required_fields: Vec<String> = Vec::new();
        let mut input_schema_obj: JsonObject<String, Value> = JsonObject::new();

        let operation_value_binding = tool_operation
            .operation_value
            .unwrap_or(serde_json::Value::Null);

        let default_operation_value = serde_json::Map::new();
        let operation_value = operation_value_binding
            .as_object()
            .filter(|obj| {
                obj.get("operationId")
                    .and_then(|id| id.as_str())
                    .map(|id| id == operation_id)
                    .unwrap_or(false)
            })
            .unwrap_or(&default_operation_value);

        //
        // Populate parameters
        //
        if let Some(parameters) = tool_operation.operation.parameters {
            let parameters_binding = operation_value
                .get("parameters")
                .unwrap_or(&serde_json::Value::Null)
                .as_array()
                .unwrap_or(&default_parameter_vec);

            for parameter in parameters {
                let parameter_binding = parameters_binding
                    .iter()
                    .find(|p| {
                        p.get("name")
                            .unwrap_or(&serde_json::Value::Null)
                            .as_str()
                            .unwrap_or("")
                            == parameter.name
                    })
                    .unwrap_or(&serde_json::Value::Null);

                if let Some(required) = parameter.required {
                    if required {
                        required_fields.push(parameter.name.clone());
                    }
                }

                let parameter_schema_binding = parameter_binding
                    .get("schema")
                    .unwrap_or(&serde_json::to_value(parameter.schema).unwrap())
                    .to_owned();

                let parameter_schema = if let Some(ref_value) =
                    parameter_schema_binding.get("$ref")
                {
                    ref_value
                } else {
                    &parameter_schema_binding
                };

                let clean_parameter = MCPParameter::new(
                    parameter.description.clone(),
                    parameter_schema.to_owned(),
                );

                //
                // Check if clean_parameter is empty
                //
                if clean_parameter.schema.is_null() {
                    continue;
                }

                //
                // Merge the clean_parameter into the input_schema_obj
                //
                input_schema_obj.insert(
                    parameter.name.clone(),
                    serde_json::to_value(clean_parameter)
                        .unwrap_or(serde_json::Value::Null),
                );
            }
        }

        //
        // Populate request body
        //
        if let Some(_) = tool_operation.operation.request_body {
            required_fields.push("body".to_string());

            let request_body_binding = operation_value
                .get("requestBody")
                .unwrap_or(&serde_json::Value::Null)
                .get("content")
                .unwrap_or(&serde_json::Value::Null)
                .get("application/json")
                .unwrap_or(&serde_json::Value::Null)
                .get("schema")
                .unwrap_or(&serde_json::Value::Null);

            let properties =
                if let Some(properties) = request_body_binding.get("$ref") {
                    properties.to_owned()
                } else {
                    request_body_binding.to_owned()
                };

            input_schema_obj.insert(
                "body".to_string(),
                json!({
                    "type": "object",
                    "properties": properties.get("properties").unwrap_or(&serde_json::Value::Null),
                    "required": properties.get("required").unwrap_or(&serde_json::Value::Null),
                }),
            );
        }

        let input_schema_value = serde_json::json!({
            "type": "object",
            "properties": input_schema_obj,
            "required": required_fields,
        });

        //
        // Convert input_schema_value into JsonObject<String, Value>
        //
        let input_schema_json_object = input_schema_value
            .as_object()
            .unwrap()
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect::<JsonObject<String, Value>>();

        Tool::new(
            tool_operation.operation.operation_id.unwrap(),
            std::borrow::Cow::Owned(
                tool_operation
                    .operation
                    .summary
                    .unwrap_or("no summary".to_string()),
            ),
            Arc::new(input_schema_json_object),
        )
    }
}

#[derive(Clone)]
pub(crate) struct MyceliumMcpHandler {
    /// The operations
    ///
    /// The original operations derived from the openapi documentation.
    ///
    operations: Arc<Vec<ToolOperation>>,
}

impl MyceliumMcpHandler {
    pub fn new(operations_database: ServiceOpenApiSchema) -> Self {
        let docs = match serde_json::to_value(operations_database.docs) {
            Ok(value) => value,
            Err(_) => serde_json::Value::Null,
        };

        let tools: Vec<ToolOperation> = operations_database
            .operations
            .iter()
            .filter_map(|tool_operation| {
                let operation_id = build_operation_id(
                    &tool_operation.method.to_string(),
                    tool_operation.operation.operation_id.as_ref(),
                    &tool_operation.service.name,
                    &tool_operation.path,
                );

                let service_docs =
                    docs.get(&tool_operation.service.name).unwrap();

                let operation = Operation {
                    operation_id: Some(operation_id),
                    ..tool_operation.operation.clone()
                };

                let mut operation_value =
                    match serde_json::to_value(operation.clone()) {
                        Ok(value) => value,
                        Err(_) => serde_json::Value::Null,
                    };

                let resolved_operation = match resolve_refs(
                    &mut operation_value,
                    0,
                    15,
                    &service_docs,
                ) {
                    Ok(value) => value,
                    Err(_) => operation_value,
                };

                Some(ToolOperation {
                    operation,
                    operation_value: Some(resolved_operation),
                    ..tool_operation.clone()
                })
            })
            .map(|tool_operation| tool_operation.clone())
            .collect::<Vec<_>>();

        Self {
            operations: Arc::new(tools),
        }
    }
}

impl ServerHandler for MyceliumMcpHandler {
    /// List all tools available in the service
    ///
    /// This method returns a list of all tools available in the service with no
    /// filters.
    ///
    fn list_tools(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ListToolsResult, ErrorData>> + Send + '_
    {
        let mut sorted_operations = self.operations.to_vec();
        sorted_operations
            .sort_by_key(|tool| tool.operation.operation_id.clone());

        let tools: Vec<Tool> = sorted_operations
            .iter()
            .map(|tool_def| tool_def.clone().into())
            .collect();

        std::future::ready(Ok(ListToolsResult::with_all_items(tools)))
    }

    /// Call a tool
    ///
    /// This method is not implemented yet.
    ///
    fn call_tool(
        &self,
        _request: CallToolRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<CallToolResult, ErrorData>> + Send + '_
    {
        std::future::ready(Err(ErrorData::method_not_found::<
            CallToolRequestMethod,
        >()))
    }
}
