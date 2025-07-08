use crate::{
    dtos::{
        generic_schema_or_ref::GenericSchemaOrRef, schema_type::SchemaType,
    },
    entities::{DepthTracker, ReferenceResolver},
};

use mycelium_base::utils::errors::{execution_err, MappedErrors};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Schema {
    #[serde(rename = "$ref", skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,

    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub r#type: Option<SchemaType>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub nullable: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<HashMap<String, GenericSchemaOrRef<Schema>>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<serde_json::Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub enum_values: Option<Vec<serde_json::Value>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,
}

impl ReferenceResolver for Schema {
    fn resolve_ref(
        &self,
        components: &serde_json::Value,
        depth_tracker: &mut DepthTracker,
    ) -> Result<serde_json::Value, MappedErrors> {
        if depth_tracker.should_stop() {
            return Ok(depth_tracker.empty_value());
        }

        depth_tracker.increment();

        if let Some(reference) = &self.reference {
            //
            // Example:
            //
            // "#/components/schemas/APIAccountType"
            //
            let ref_path = reference.split('/').collect::<Vec<&str>>();

            //
            // Here the element is defined. Possible values are:
            //
            // - schemas
            // - responses
            // - parameters
            // - examples
            // - requestBodies
            // - headers
            // - securitySchemes
            //
            // Element is the penultimate element of the ref_path
            //
            // Example:
            //
            // The path: "#/components/schemas/APIAccountType"
            // The element is: "schemas"
            //
            let element_definition = if ref_path.len() > 2 {
                ref_path[ref_path.len() - 2]
            } else {
                return Ok(depth_tracker.empty_value());
            };

            //
            // The element name is the last element of the ref_path
            //
            // Example:
            //
            // The path: "#/components/schemas/APIAccountType"
            // The element name is: "APIAccountType"
            //
            let element_name = ref_path.last().ok_or(execution_err(format!(
                "Failed to resolve schema ref. Unable to get the component name from reference: {reference}"
            )))?;

            // Find in nested components
            //
            // Components should be a map with keys:
            //
            // We need to traverse the ref_path to find the value.
            //
            let ref_value = components
                .get(element_definition)
                .and_then(|schema| schema.get(element_name))
                .ok_or(execution_err(format!(
                    "Failed to resolve schema ref: {element_name}"
                )))?;

            return Ok(ref_value.clone());
        }

        serde_json::to_value(self).map_err(|e| {
            execution_err(format!("Failed to serialize schema: {e}"))
        })
    }
}
