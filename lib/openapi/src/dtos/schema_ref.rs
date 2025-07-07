use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct SchemaRef {
    #[serde(rename = "$ref")]
    pub reference: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}
