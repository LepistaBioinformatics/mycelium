use crate::dtos::{generic_schema_or_ref::GenericSchemaOrRef, schema::Schema};

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Header {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<GenericSchemaOrRef<Schema>>,
}
