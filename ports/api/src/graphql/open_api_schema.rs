use async_graphql::{Enum, SimpleObject};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ? ---------------------------------------------------------------------------
// ? Reference fields
// ? ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, Serialize, SimpleObject, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Reference {
    #[serde(alias = "$ref", skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, SimpleObject, Clone)]
#[graphql(name = "SchemaOrRefSchema")]
#[serde(rename_all = "camelCase")]
pub struct SchemaOrRefSchema {
    #[serde(flatten)]
    pub reference: Reference,

    #[serde(flatten)]
    pub schema: Option<Schema>,
}

#[derive(Debug, Deserialize, Serialize, SimpleObject, Clone)]
#[graphql(name = "SchemaOrRefRequestBody")]
#[serde(rename_all = "camelCase")]
pub struct SchemaOrRefRequestBody {
    #[serde(flatten)]
    pub reference: Reference,

    #[serde(flatten)]
    pub schema: Option<RequestBody>,
}

#[derive(Debug, Deserialize, Serialize, SimpleObject, Clone)]
#[graphql(name = "SchemaOrRefResponse")]
#[serde(rename_all = "camelCase")]
pub struct SchemaOrRefResponse {
    #[serde(flatten)]
    pub reference: Reference,

    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub schema: Option<Response>,
}

#[derive(Debug, Deserialize, Serialize, SimpleObject, Clone)]
#[graphql(name = "SchemaOrRefParameter")]
#[serde(rename_all = "camelCase")]
pub struct SchemaOrRefParameter {
    #[serde(flatten)]
    pub reference: Reference,

    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub schema: Option<Parameter>,
}

#[derive(Debug, Deserialize, Serialize, SimpleObject, Clone)]
#[graphql(name = "SchemaOrRefHeader")]
#[serde(rename_all = "camelCase")]
pub struct SchemaOrRefHeader {
    #[serde(flatten)]
    pub reference: Reference,

    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub schema: Option<Header>,
}

#[derive(Debug, Deserialize, Serialize, SimpleObject, Clone)]
#[graphql(name = "SchemaOrRefExample")]
#[serde(rename_all = "camelCase")]
pub(crate) struct SchemaOrRefExample {
    #[serde(flatten)]
    pub reference: Reference,

    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub schema: Option<Example>,
}

// ? ---------------------------------------------------------------------------
// ? OpenApiPartial
// ? ---------------------------------------------------------------------------

#[derive(Clone, Debug, Deserialize, Serialize, SimpleObject)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OpenApiPartial {
    pub paths: HashMap<String, HashMap<String, Option<Operation>>>,
    pub components: Components,
}

#[derive(Debug, Deserialize, Serialize, SimpleObject, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Operation {
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
    #[graphql(name = "requestBody")]
    pub request_body: Option<SchemaOrRefRequestBody>,

    #[serde(default)]
    #[graphql(name = "responses")]
    pub responses: HashMap<String, SchemaOrRefResponse>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deprecated: Option<bool>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub security: Option<Vec<HashMap<String, Vec<String>>>>,
}

#[derive(Debug, Deserialize, Serialize, Copy, Clone, Enum, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) enum Location {
    Query,
    Path,
    Header,
    Cookie,
}

#[derive(Debug, Deserialize, Serialize, SimpleObject, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Parameter {
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

    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[graphql(name = "schema")]
    pub schema: Option<SchemaOrRefSchema>,
}

#[derive(Debug, Deserialize, Serialize, SimpleObject, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RequestBody {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(default)]
    pub content: HashMap<String, MediaType>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize, SimpleObject, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MediaType {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[graphql(name = "schema")]
    pub schema: Option<SchemaOrRefSchema>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub example: Option<serde_json::Value>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[graphql(name = "examples")]
    pub examples: Option<HashMap<String, SchemaOrRefExample>>,
}

#[derive(Debug, Deserialize, Serialize, SimpleObject, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Response {
    #[serde(default)]
    pub description: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[graphql(name = "headers")]
    pub headers: Option<HashMap<String, SchemaOrRefHeader>>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<HashMap<String, MediaType>>,
}

#[derive(Debug, Deserialize, Serialize, SimpleObject, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Components {
    #[serde(default)]
    pub schemas: HashMap<String, SchemaOrRefSchema>,

    #[serde(default)]
    pub responses: HashMap<String, SchemaOrRefResponse>,

    #[serde(default)]
    pub parameters: HashMap<String, SchemaOrRefParameter>,

    #[serde(default)]
    pub request_bodies: HashMap<String, SchemaOrRefRequestBody>,

    #[serde(default)]
    pub headers: HashMap<String, SchemaOrRefHeader>,

    #[serde(default)]
    pub examples: HashMap<String, SchemaOrRefExample>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged, rename_all = "camelCase")]
enum SchemaType {
    String(String),
    Array(Vec<String>),
}

impl SchemaType {
    pub fn as_vec(&self) -> Vec<String> {
        match self {
            SchemaType::String(s) => vec![s.clone()],
            SchemaType::Array(v) => v.clone(),
        }
    }
}

#[derive(Serialize, Debug, Clone, SimpleObject)]
pub(crate) struct SchemaTypeGQL {
    pub values: Vec<String>,
}

impl<'de> Deserialize<'de> for SchemaTypeGQL {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let values: SchemaType = serde::Deserialize::deserialize(deserializer)?;

        Ok(SchemaTypeGQL {
            values: values.as_vec(),
        })
    }
}

impl From<&SchemaType> for SchemaTypeGQL {
    fn from(value: &SchemaType) -> Self {
        SchemaTypeGQL {
            values: value.as_vec(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, SimpleObject, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ItemType {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item: Option<Box<SchemaOrRefSchema>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub boolean: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize, SimpleObject, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Schema {
    #[serde(alias = "type", skip_serializing_if = "Option::is_none")]
    #[graphql(name = "type")]
    pub schema_type: Option<SchemaTypeGQL>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nullable: Option<bool>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub properties: Option<HashMap<String, SchemaOrRefSchema>>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub items: Option<ItemType>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enum_values: Option<Vec<serde_json::Value>>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, Serialize, SimpleObject, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Example {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, Serialize, SimpleObject, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Header {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[graphql(name = "schema")]
    pub schema: Option<SchemaOrRefSchema>,
}
