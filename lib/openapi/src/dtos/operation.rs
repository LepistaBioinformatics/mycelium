use crate::{
    dtos::{content::Content, parameter::Parameter},
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
    pub request_body: Option<Content>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub responses: Option<HashMap<String, Content>>,

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

        //
        // Resolve the parameters
        //
        if let Some(parameters) = self.parameters.as_ref() {
            value["parameters"] = if let false = parameters.is_empty() {
                parameters
                    .iter()
                    .filter_map(|parameter| {
                        //
                        // Clone the depth tracker to guard against parallel
                        // increment
                        //
                        let mut tracker_clone = depth_tracker.clone();

                        //
                        // Resolve the parameter
                        //
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
                    .into()
            } else {
                depth_tracker.empty_value()
            };
        }

        //
        // Resolve the request body
        //
        if let Some(request_body) = self.request_body.as_ref() {
            value["requestBody"] =
                request_body.resolve_ref(components, depth_tracker)?;
        }

        //
        // Resolve the responses
        //
        if let Some(responses) = self.responses.as_ref() {
            let mut responses_value = serde_json::Map::new();

            for (key, response) in responses {
                let resolved =
                    response.resolve_ref(components, depth_tracker)?;

                responses_value.insert(key.clone(), resolved);
            }

            value["responses"] = serde_json::Value::Object(responses_value);
        }

        Ok(value)
    }
}
