use crate::dtos::{
    generic_schema_or_ref::GenericSchemaOrRef, schema::Schema,
    schema_or_ref::SchemaOrRef,
};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SchemaOrOneOf {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(flatten)]
    pub schema: Option<SchemaOrRef>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub one_of: Option<Vec<GenericSchemaOrRef<Schema>>>,
}
