use crate::{
    dtos::{location::Location, schema_or_one_of::SchemaOrOneOf},
    entities::{DepthTracker, ReferenceResolver},
};

use mycelium_base::utils::errors::{execution_err, MappedErrors};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Parameter {
    #[serde(default)]
    pub name: String,

    #[serde(rename = "in")]
    pub r#in: Location,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deprecated: Option<bool>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allow_empty_value: Option<bool>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub explode: Option<bool>,

    pub schema: SchemaOrOneOf,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub security: Option<serde_json::Value>,
}

impl ReferenceResolver for Parameter {
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
            execution_err(format!("Failed to resolve parameter: {e}"))
        })?;

        let schema = self.schema.resolve_ref(components, depth_tracker)?;

        value["schema"] = schema;

        Ok(value)
    }
}
