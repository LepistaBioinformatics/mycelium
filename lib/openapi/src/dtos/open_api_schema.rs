use crate::dtos::operation::Operation;

use mycelium_base::utils::errors::{execution_err, MappedErrors};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OpenApiSchema {
    pub openapi: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub info: Option<serde_json::Value>,

    #[serde(default)]
    pub paths: HashMap<String, HashMap<String, Option<Operation>>>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub components: Option<serde_json::Value>,
}

impl OpenApiSchema {
    #[tracing::instrument(name = "load_doc_from_string", skip_all)]
    fn load_doc_from_string(
        content: &str,
    ) -> Result<OpenApiSchema, MappedErrors> {
        let doc =
            serde_json::from_str::<OpenApiSchema>(&content).map_err(|e| {
                execution_err(format!("Failed to parse OpenAPI document: {e}"))
            })?;

        Ok(doc)
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
        assert!(doc.is_ok());

        let example_doc =
            OpenApiSchema::load_doc_from_string(get_spec_example_file());

        assert!(example_doc.is_ok());

        // Test if the loaded document is the same as the example document
        assert_eq!(doc.unwrap(), example_doc.unwrap());
    }
}
