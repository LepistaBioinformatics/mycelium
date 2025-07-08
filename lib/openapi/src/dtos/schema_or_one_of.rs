use crate::{
    dtos::schema::Schema,
    entities::{DepthTracker, ReferenceResolver},
};

use mycelium_base::utils::errors::{execution_err, MappedErrors};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SchemaOrOneOf {
    #[serde(skip_serializing_if = "Option::is_none")]
    one_of: Option<Vec<Schema>>,

    #[serde(flatten)]
    schema: Schema,
}

impl ReferenceResolver for SchemaOrOneOf {
    fn resolve_ref(
        &self,
        components: &serde_json::Value,
        depth_tracker: &mut DepthTracker,
    ) -> Result<serde_json::Value, MappedErrors> {
        if depth_tracker.should_stop() {
            return Ok(depth_tracker.empty_value());
        }

        depth_tracker.increment();

        if let Some(one_of) = &self.one_of {
            let one_of_resolved = one_of
                .iter()
                .filter_map(|item| {
                    let mut tracker_clone = depth_tracker.clone();

                    item.resolve_ref(components, &mut tracker_clone)
                        .map_err(|e| {
                            execution_err(format!(
                                "Failed to resolve schema: {e}"
                            ))
                        })
                        .ok()
                })
                .collect::<Vec<_>>();

            if one_of_resolved.is_empty() {
                return Ok(depth_tracker.empty_value());
            }

            return Ok(one_of_resolved.into());
        }

        return self.schema.resolve_ref(components, depth_tracker);
    }
}
