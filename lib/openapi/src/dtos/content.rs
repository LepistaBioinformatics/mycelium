use crate::{
    dtos::schema::Schema,
    entities::{DepthTracker, ReferenceResolver},
};

use mycelium_base::utils::errors::{execution_err, MappedErrors};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ValueSchema {
    schema: Schema,
}

impl ReferenceResolver for ValueSchema {
    fn resolve_ref(
        &self,
        components: &serde_json::Value,
        depth_tracker: &mut DepthTracker,
    ) -> Result<serde_json::Value, MappedErrors> {
        return self.schema.resolve_ref(components, depth_tracker);
    }
}

/// Content Schema
///
/// Should used to represent the content of the response.
///
/// Example:
///
/// ```json
/// {
///     "content": {
///         "application/json": {
///             "schema": {
///                 "type": "object"
///                 "properties": {
///                     "name": {
///                         "type": "string"
///                     }
///                 }
///             }
///         }
///     },
///     "description": "The response content",
///     "required": true
/// }
/// ```
///
/// or, using a reference to a schema:
///
/// ```json
/// {
///     "content": {
///         "application/json": {
///             "schema": {
///                 "$ref": "#/components/schemas/HttpJsonResponse"
///             }
///         }
///     },
///     "description": "The response content",
///     "required": true
/// }
/// ```
///
/// This struct refers to the value of the schema in response.
///
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Content {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    content: Option<HashMap<String, ValueSchema>>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    description: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    required: Option<bool>,
}

impl ReferenceResolver for Content {
    fn resolve_ref(
        &self,
        components: &serde_json::Value,
        depth_tracker: &mut DepthTracker,
    ) -> Result<serde_json::Value, MappedErrors> {
        if depth_tracker.should_stop() {
            return Ok(depth_tracker.empty_value());
        }

        depth_tracker.increment();

        let mut value = serde_json::to_value(self).map_err(|e| {
            execution_err(format!("Failed to serialize content: {e}"))
        })?;

        if let Some(content) = self.content.as_ref() {
            let content = content
                .iter()
                .map(|(key, value)| {
                    (
                        key,
                        value.resolve_ref(components, depth_tracker).map_err(
                            |e| {
                                execution_err(format!(
                                    "Failed to resolve content: {e}"
                                ))
                            },
                        ),
                    )
                })
                .filter(|(_, value)| value.is_ok())
                .map(|(key, value)| (key.clone(), value.unwrap()))
                .collect::<HashMap<String, serde_json::Value>>();

            value["content"] = serde_json::to_value(content).map_err(|e| {
                execution_err(format!("Failed to serialize content: {e}"))
            })?;
        }

        Ok(value)
    }
}
