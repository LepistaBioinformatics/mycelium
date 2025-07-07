use crate::dtos::parameter::Parameter;

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
