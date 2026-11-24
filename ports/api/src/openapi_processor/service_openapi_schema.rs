use crate::dtos::ToolOperation;

use mycelium_base::utils::errors::{execution_err, MappedErrors};
use mycelium_openapi::OpenApiSchema;
use serde_json::Value;
use std::collections::HashMap;

/// The in-memory catalog of discoverable operations
///
/// Operations are sorted once and every service document is serialized to
/// JSON once, when the catalog is built, so listing requests only read it.
///
#[derive(Debug, Clone)]
pub struct ServiceOpenApiSchema {
    /// The operations, sorted by operation id, method, path and service
    pub operations: Vec<ToolOperation>,

    /// The OpenAPI documents indexed by service name
    pub docs: HashMap<String, OpenApiSchema>,

    /// The OpenAPI documents serialized to JSON, indexed by service name
    pub serialized_docs: HashMap<String, Value>,
}

impl ServiceOpenApiSchema {
    pub(crate) fn new(
        mut operations: Vec<ToolOperation>,
        docs: HashMap<String, OpenApiSchema>,
    ) -> Result<Self, MappedErrors> {
        sort_operations(&mut operations);
        let serialized_docs = serialize_docs(&docs)?;

        Ok(Self {
            operations,
            docs,
            serialized_docs,
        })
    }
}

fn sort_operations(operations: &mut [ToolOperation]) {
    operations.sort_by(|first, second| {
        first
            .operation
            .operation_id
            .cmp(&second.operation.operation_id)
            .then_with(|| {
                first.method.to_string().cmp(&second.method.to_string())
            })
            .then_with(|| first.path.cmp(&second.path))
            .then_with(|| first.service.name.cmp(&second.service.name))
    });
}

fn serialize_docs(
    docs: &HashMap<String, OpenApiSchema>,
) -> Result<HashMap<String, Value>, MappedErrors> {
    docs.iter()
        .map(|(service_name, document)| {
            let serialized =
                serde_json::to_value(document).map_err(|error| {
                    execution_err(format!(
                        "Failed to serialize the OpenAPI document of \
                     {service_name}: {error}"
                    ))
                })?;

            Ok((service_name.to_owned(), serialized))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::openapi_processor::synthetic_catalog_fixture::build_synthetic_catalog;

    #[test]
    fn new_sorts_operations_and_serializes_every_document(
    ) -> Result<(), MappedErrors> {
        let catalog = build_synthetic_catalog(2, 3);
        let mut operations = catalog.operations;
        operations.reverse();

        let schema = ServiceOpenApiSchema::new(operations, catalog.docs)?;

        let operation_ids = schema
            .operations
            .iter()
            .map(|operation| operation.operation.operation_id.clone())
            .collect::<Vec<_>>();
        let mut sorted_ids = operation_ids.clone();
        sorted_ids.sort();

        assert_eq!(operation_ids, sorted_ids);
        assert_eq!(schema.serialized_docs.len(), 2);
        assert!(schema.serialized_docs["service-01"]["components"].is_object());

        Ok(())
    }
}
