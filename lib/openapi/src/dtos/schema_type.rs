use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq, ToSchema)]
#[serde(untagged, rename_all = "camelCase")]
pub enum SchemaType {
    String(String),
    Array(Vec<String>),
}
