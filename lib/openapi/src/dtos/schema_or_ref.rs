use crate::dtos::{schema::Schema, schema_ref::SchemaRef};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(untagged, rename_all = "camelCase")]
pub enum SchemaOrRef {
    Schema(Schema),
    Ref(SchemaRef),
}
