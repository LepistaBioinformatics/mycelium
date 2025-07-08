use crate::entities::{DepthTracker, ReferenceResolver};

use mycelium_base::utils::errors::{execution_err, MappedErrors};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct SchemaRef {
    #[serde(flatten, rename = "$ref")]
    pub reference: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl ReferenceResolver for SchemaRef {
    fn resolve_ref(
        &self,
        components: &serde_json::Value,
        depth_tracker: &mut DepthTracker,
    ) -> Result<serde_json::Value, MappedErrors> {
        if depth_tracker.should_stop() {
            return Ok(depth_tracker.empty_value());
        }

        depth_tracker.increment();

        let ref_path = self.reference.split('/').collect::<Vec<&str>>();
        println!("ref_path 1: {:?}", ref_path);
        let ref_path = ref_path.last().unwrap();
        println!("ref_path 2: {:?}", ref_path);

        // Find in nested components
        //
        // Components should be a map with keys:
        // - schemas
        // - responses
        // - parameters
        // - examples
        // - requestBodies
        // - headers
        // - securitySchemes
        //
        // We need to traverse the ref_path to find the value.
        //
        let mut current_value = components;

        for path_part in ref_path.split('/') {
            println!("path_part: {:?}", path_part);

            current_value =
                current_value.get(path_part).ok_or(execution_err(format!(
                    "Failed to resolve schema ref: {ref_path}"
                )))?;
        }

        let ref_value = current_value.clone();

        Ok(ref_value.clone())
    }
}
