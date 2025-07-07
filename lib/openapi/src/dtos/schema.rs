use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::dtos::{
    generic_schema_or_ref::GenericSchemaOrRef, item_type::ItemType,
    schema_type::SchemaType,
};

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Schema {
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub r#type: Option<SchemaType>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nullable: Option<bool>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub properties: Option<HashMap<String, GenericSchemaOrRef<Schema>>>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub items: Option<ItemType>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enum_values: Option<Vec<serde_json::Value>>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,
}
