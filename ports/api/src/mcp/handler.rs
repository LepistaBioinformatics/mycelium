use crate::{
    dtos::ToolOperation,
    openapi_processor::{build_operation_id, ServiceOpenApiSchema},
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
use serde_json::{json, Map as JsonObject, Value};
use std::future::Future;
use std::sync::Arc;

impl From<ToolOperation> for Tool {
    fn from(tool_operation: ToolOperation) -> Self {
        //
        // Initialize required fields
        //
        let mut required_fields: Vec<String> = Vec::new();

        //
        // Create the input schema
        //
        // The input schema is a json schema object that describes the
        // parameters of the tool. It should contains the parameters plus the
        // request body when available.
        //
        let mut input_schema_obj: JsonObject<String, Value> = JsonObject::new();

        if let Some(parameters) = tool_operation.operation.parameters {
            for parameter in parameters {
                if let Some(required) = parameter.required {
                    if required {
                        required_fields.push(parameter.name.clone());
                    }
                }

                let clean_parameter = json!({
                    "name": parameter.name,
                    "description": parameter.description,
                    "schema": parameter.schema,
                });

                input_schema_obj.insert(
                    parameter.name.clone(),
                    serde_json::to_value(clean_parameter).unwrap(),
                );
            }
        }

        if let Some(request_body) = tool_operation.operation.request_body {
            required_fields.push("body".to_string());

            for (_, value_schema) in request_body.content.unwrap() {
                let clean_request_body = json!({
                    "description": request_body.description,
                    "schema": value_schema,
                });

                input_schema_obj.insert(
                    "body".to_string(),
                    serde_json::to_value(clean_request_body).unwrap(),
                );
            }
        }

        if let Some(security) = tool_operation.operation.security {
            println!("security: {:?}", security);
            /* if let Some(security_schema) = security.security_schema {
                input_schema_obj.extend(security_schema.into_iter().map(
                    |(key, value_schema)| {
                        (key, serde_json::to_value(value_schema).unwrap())
                    },
                ));
            } */
        }

        for (key, value) in input_schema_obj.iter() {
            if value.is_object() {
                //
                // Required is a boolean field that indicates if the field is
                // required.
                //
                if let Some(required) = value.get("required") {
                    if required.as_bool().is_some()
                        && required.as_bool().unwrap()
                    {
                        required_fields.push(key.clone());
                    }
                }
            }
        }

        let mut input_schema_value = serde_json::json!({
            "type": "object",
        });

        if input_schema_obj.len() > 0 {
            input_schema_value["properties"] =
                serde_json::to_value(input_schema_obj).unwrap();
        }

        if required_fields.len() > 0 {
            input_schema_value["required"] =
                serde_json::to_value(required_fields).unwrap();
        }

        //
        // Convert input_schema_value into JsonObject<String, Value>
        //
        let input_schema_json_object: JsonObject<String, Value> =
            serde_json::from_value(input_schema_value).unwrap();

        //println!("input_schema_json_object: {:?}", input_schema_json_object);

        //println!("input_schema_obj: {:?}", input_schema_obj);

        Tool::new(
            tool_operation.operation.operation_id.unwrap(),
            std::borrow::Cow::Owned(tool_operation.operation.summary.unwrap()),
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

                let operation = Operation {
                    operation_id: Some(operation_id),
                    ..tool_operation.operation.clone()
                };

                Some(ToolOperation {
                    operation,
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
    fn list_tools(
        &self,
        request: Option<PaginatedRequestParam>,
        context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ListToolsResult, ErrorData>> + Send + '_
    {
        let tools: Vec<Tool> = self
            .operations
            .iter()
            .map(|tool_def| tool_def.clone().into())
            .collect();

        std::future::ready(Ok(ListToolsResult::with_all_items(tools)))
    }

    fn call_tool(
        &self,
        request: CallToolRequestParam,
        context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<CallToolResult, ErrorData>> + Send + '_
    {
        std::future::ready(Err(ErrorData::method_not_found::<
            CallToolRequestMethod,
        >()))
    }
}
