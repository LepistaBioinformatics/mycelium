use crate::{
    dtos::{example::Example, generic_schema_or_ref::GenericSchemaOrRef},
    entities::{DepthTracker, ReferenceResolver},
};

use mycelium_base::utils::errors::MappedErrors;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Content Schema
///
/// Should used to represent the content of the response.
///
/// Example:
///
/// ```json
/// {
///     "application/json": {
///         "schema": {
///             "type": "object"
///             "properties": {
///                 "name": {
///                     "type": "string"
///                 }
///             }
///         },
///         "example": {
///             "name": "John Doe"
///         }
///     }
/// }
/// ```
///
/// or, using a reference to a schema:
///
/// ```json
/// {
///     "application/json": {
///         "schema": {
///             "$ref": "#/components/schemas/HttpJsonResponse"
///         }
///     }
/// }
/// ```
///
/// This struct refers to the value of the schema in response.
///
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ContentValueSchema {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<GenericSchemaOrRef<serde_json::Value>>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub example: Option<serde_json::Value>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub examples: Option<HashMap<String, GenericSchemaOrRef<Example>>>,
}

impl ReferenceResolver for ContentValueSchema {
    fn resolve_ref(
        &self,
        components: &serde_json::Value,
        depth_tracker: &mut DepthTracker,
    ) -> Result<serde_json::Value, MappedErrors> {
        if self.schema.is_some() {
            return self
                .schema
                .as_ref()
                .unwrap()
                .resolve_ref(components, depth_tracker);
        }

        Ok(serde_json::Value::Null)
    }
}
