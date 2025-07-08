use crate::{
    dtos::parameter::Parameter,
    entities::{DepthTracker, ReferenceResolver},
};

use mycelium_base::utils::errors::{execution_err, MappedErrors};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Operation {
    #[serde(default)]
    pub operation_id: Option<String>,

    #[serde(default)]
    pub tags: Vec<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parameters: Option<Vec<Parameter>>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_body: Option<serde_json::Value>,

    #[serde(default)]
    pub responses: HashMap<String, serde_json::Value>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deprecated: Option<bool>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub security: Option<serde_json::Value>,
}

impl ReferenceResolver for Operation {
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
            execution_err(format!("Failed to resolve operation: {e}"))
        })?;

        let parameters = self.parameters.as_ref().map(|params| {
            params
                .iter()
                .filter_map(|parameter| {
                    let mut tracker_clone = depth_tracker.clone();

                    parameter
                        .resolve_ref(components, &mut tracker_clone)
                        .map_err(|e| {
                            execution_err(format!(
                                "Failed to resolve parameter: {e}"
                            ))
                        })
                        .ok()
                })
                .collect::<Vec<serde_json::Value>>()
        });

        if let Some(parameters) = parameters {
            if !parameters.is_empty() {
                value["parameters"] = parameters.into();
            }
        }

        Ok(value)
    }
}
