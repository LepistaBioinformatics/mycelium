use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(untagged, rename_all = "camelCase")]
pub enum SchemaType {
    String(String),
    Array(Vec<String>),
}
