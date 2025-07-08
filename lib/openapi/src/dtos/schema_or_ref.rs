use crate::{
    dtos::{schema::Schema, schema_ref::SchemaRef},
    entities::{DepthTracker, ReferenceResolver},
};

use mycelium_base::utils::errors::{execution_err, MappedErrors};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(untagged, rename_all = "camelCase")]
pub enum SchemaOrRef {
    Schema(Schema),
    Ref(SchemaRef),
}

impl ReferenceResolver for SchemaOrRef {
    fn resolve_ref(
        &self,
        components: &serde_json::Value,
        depth_tracker: &mut DepthTracker,
    ) -> Result<serde_json::Value, MappedErrors> {
        if depth_tracker.should_stop() {
            return Ok(depth_tracker.empty_value());
        }

        depth_tracker.increment();

        println!("\n\nresolving schema or ref: {:?}", self);
        println!("depth_tracker: {:?}", depth_tracker);

        match self {
            SchemaOrRef::Schema(schema) => serde_json::to_value(schema)
                .map_err(|e| {
                    execution_err(format!(
                        "Failed to convert schema to value: {e}"
                    ))
                }),
            SchemaOrRef::Ref(schema_ref) => {
                println!("resolving ref: {:?}", schema_ref);
                schema_ref.resolve_ref(components, depth_tracker)
            }
        }
    }
}
