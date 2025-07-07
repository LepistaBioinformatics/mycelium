use crate::dtos::{location::Location, schema_or_one_of::SchemaOrOneOf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
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
}
