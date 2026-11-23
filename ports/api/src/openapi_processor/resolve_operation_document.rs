use crate::{
    dtos::ToolOperation,
    openapi_processor::{
        build_operation_id, resolve_refs,
        service_openapi_schema::ServiceOpenApiSchema,
    },
};

use mycelium_base::utils::errors::{execution_err, MappedErrors};
use serde_json::{Map, Value};

const MAX_RESOLUTION_PASSES: usize = 3;
const MAX_RESOLUTION_DEPTH: usize = 15;

/// Resolve the document of a listed operation
///
/// Serialize the tool operation, merge its resolved inputs (parameters,
/// request body, ...) when the service document provides them, and resolve
/// the remaining component references in place. The record carries the
/// operation only, never the whole service document.
///
pub(crate) fn resolve_operation_document(
    tool_operation: &ToolOperation,
    catalog: &ServiceOpenApiSchema,
) -> Result<Value, MappedErrors> {
    let mut record = serde_json::to_value(tool_operation).map_err(|error| {
        execution_err(format!("Failed to serialize the operation: {error}"))
    })?;

    merge_resolved_inputs(&mut record, tool_operation, catalog);

    let service_document =
        catalog.serialized_docs.get(&tool_operation.service.name);
    resolve_component_refs(&mut record, service_document);

    record["operationId"] = build_operation_id(
        &tool_operation.method.to_string(),
        tool_operation.operation.operation_id.as_ref(),
        &tool_operation.service.name,
        &tool_operation.path,
    )
    .into();

    Ok(record)
}

fn merge_resolved_inputs(
    record: &mut Value,
    tool_operation: &ToolOperation,
    catalog: &ServiceOpenApiSchema,
) {
    let Some(resolved_inputs) = resolve_input_refs(tool_operation, catalog)
    else {
        return;
    };

    let Value::Object(record_fields) = record else {
        return;
    };

    record_fields.extend(resolved_inputs);
}

fn resolve_input_refs(
    tool_operation: &ToolOperation,
    catalog: &ServiceOpenApiSchema,
) -> Option<Map<String, Value>> {
    let operation_id = tool_operation.operation.operation_id.as_ref()?;
    let document = catalog.docs.get(&tool_operation.service.name)?;

    match document
        .resolve_input_refs_from_operation_id(operation_id)
        .ok()?
    {
        Value::Object(resolved_inputs) => Some(resolved_inputs),
        _ => None,
    }
}

fn resolve_component_refs(record: &mut Value, document: Option<&Value>) {
    let Some(document) = document else {
        return;
    };

    for _ in 0..MAX_RESOLUTION_PASSES {
        let Err(error) =
            resolve_refs(record, 0, MAX_RESOLUTION_DEPTH, document)
        else {
            continue;
        };

        tracing::debug!("Unable to resolve every operation ref: {error}");
    }
}
