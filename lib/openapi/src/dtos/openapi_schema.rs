use crate::{
    dtos::operation::Operation,
    entities::{DepthTracker, ReferenceResolver},
};

use mycelium_base::utils::errors::{execution_err, MappedErrors};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct MethodOperation {
    /// The operations
    ///
    /// This is the operations of the OpenAPI specification.
    ///
    /// Example:
    ///
    /// ```json
    /// {
    ///     "get": {
    ///         "operationId": "get_record"
    ///     }
    /// }
    /// ```
    #[serde(default, flatten)]
    pub operations: HashMap<String, Operation>,
}

impl MethodOperation {
    /// Find an operation by operation id
    ///
    /// This function finds an operation by operation id.
    ///
    pub fn find_operation(&self, operation_id: &str) -> Option<&Operation> {
        self.operations.values().find(|operation| {
            operation.operation_id == Some(operation_id.to_string())
        })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Paths {
    /// The paths
    ///
    /// This is the paths of the OpenAPI specification.
    ///
    /// Example:
    ///
    /// ```json
    /// {
    ///     "/path/to/route": {
    ///         "get": {
    ///             "operationId": "get_record"
    ///         },
    ///         "post": {
    ///             "operationId": "create_record"
    ///         }
    ///     }
    /// }
    /// ```
    #[serde(default, flatten)]
    pub paths: HashMap<String, MethodOperation>,
}

impl Paths {
    /// Find an operation by operation id
    ///
    /// This function finds an operation by operation id.
    ///
    pub fn find_operation(&self, operation_id: &str) -> Option<&Operation> {
        self.paths
            .values()
            .find_map(|path| path.find_operation(operation_id))
    }
}

/// OpenAPI schema
///
/// This is the main schema for the OpenAPI specification.
///
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OpenApiSchema {
    /// The OpenAPI version
    ///
    /// This is the version of the OpenAPI specification.
    ///
    pub openapi: String,

    /// The info
    ///
    /// This is the info of the OpenAPI specification.
    ///
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub info: Option<serde_json::Value>,

    /// The paths
    ///
    /// This is the paths of the OpenAPI specification.
    ///
    /// Paths are indexed by route and method.
    ///
    /// Example:
    ///
    /// ```json
    /// {
    ///     "/path/to/route": {
    ///         "get": {
    ///             "operationId": "get_record"
    ///         },
    ///         "post": {
    ///             "operationId": "create_record"
    ///         }
    ///     }
    /// }
    /// ```
    ///
    #[serde(default)]
    pub paths: Paths,

    /// The components
    ///
    /// This is the components of the OpenAPI specification.
    ///
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub components: Option<serde_json::Value>,

    /// The security
    ///
    /// This is the security of the OpenAPI specification.
    ///
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub security: Option<serde_json::Value>,
}

impl OpenApiSchema {
    #[tracing::instrument(name = "load_doc_from_string", skip_all)]
    pub fn load_doc_from_string(
        content: &str,
    ) -> Result<OpenApiSchema, MappedErrors> {
        let doc =
            serde_json::from_str::<OpenApiSchema>(&content).map_err(|e| {
                execution_err(format!("Failed to parse OpenAPI document: {e}"))
            })?;

        Ok(doc)
    }

    /// Resolve the input refs
    ///
    /// This function resolves the references from input elements like
    /// parameters, request bodies, headers, etc.
    ///
    /// Client methods should simple call this method with the operation id
    /// and the input element name.
    ///
    #[tracing::instrument(
        name = "resolve_input_refs_from_operation_id",
        skip_all
    )]
    pub fn resolve_input_refs_from_operation_id(
        &self,
        operation_id: &str,
    ) -> Result<serde_json::Value, MappedErrors> {
        let operation = self.paths.find_operation(operation_id);

        let operation = operation.ok_or(execution_err(format!(
            "Operation {operation_id} not found"
        )))?;

        let mut depth_tracker = DepthTracker::new(25);

        let resolved_operation = operation.resolve_ref(
            &self.components.clone().unwrap_or_default(),
            &mut depth_tracker,
        )?;

        Ok(resolved_operation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Get the example OpenAPI spec file
    ///
    /// This is used to make the JSON object deterministic.
    ///
    fn get_spec_example_file() -> &'static str {
        include_str!("./mock/example-openapi.json")
    }

    #[test]
    fn test_load_doc_from_string() {
        let doc = OpenApiSchema::load_doc_from_string(get_spec_example_file());

        if doc.is_err() {
            println!("doc: {:?}", doc);
        }

        assert!(doc.is_ok());

        let example_doc =
            OpenApiSchema::load_doc_from_string(get_spec_example_file());

        assert!(example_doc.is_ok());

        let doc = doc.unwrap();

        // Test if the loaded document is the same as the example document
        assert_eq!(doc, example_doc.unwrap());
    }

    #[test]
    fn test_resolve_input_refs_from_operation_id() {
        let doc = OpenApiSchema::load_doc_from_string(get_spec_example_file());

        if doc.is_err() {
            println!("doc: {:?}", doc);
        }

        assert!(doc.is_ok());

        let doc = doc.unwrap();

        for operation_id in
            ["register_tenant_tag_url", "list_accounts_by_type_url"]
        {
            let operation =
                doc.resolve_input_refs_from_operation_id(operation_id);

            assert!(operation.is_ok());
        }
    }
}
