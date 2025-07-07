use crate::dtos::{generic_schema_or_ref::GenericSchemaOrRef, schema::Schema};

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ItemType {
    Item(Option<Box<GenericSchemaOrRef<Schema>>>),
    Boolean(Option<bool>),
}
