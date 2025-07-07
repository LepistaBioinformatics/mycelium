use crate::dtos::schema_ref::SchemaRef;

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GenericSchemaOrRef<T>
where
    T: Serialize,
{
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub reference: Option<SchemaRef>,

    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub schema: Option<T>,
}
