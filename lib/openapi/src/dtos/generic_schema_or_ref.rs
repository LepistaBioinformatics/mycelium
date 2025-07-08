use crate::{
    dtos::schema_ref::SchemaRef,
    entities::{DepthTracker, ReferenceResolver},
};

use mycelium_base::utils::errors::{execution_err, MappedErrors};
use serde::{Deserialize, Serialize};
use std::fmt::Debug as FmtDebug;

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GenericSchemaOrRef<T>
where
    T: Serialize + Clone,
{
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub reference: Option<SchemaRef>,

    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub schema: Option<T>,
}

impl<T: Serialize + Clone + FmtDebug> ReferenceResolver
    for GenericSchemaOrRef<T>
{
    fn resolve_ref(
        &self,
        components: &serde_json::Value,
        depth_tracker: &mut DepthTracker,
    ) -> Result<serde_json::Value, MappedErrors> {
        if depth_tracker.should_stop() {
            return Ok(depth_tracker.empty_value());
        }

        depth_tracker.increment();

        println!("GenericSchemaOrRef::resolve_ref: {:?}", self);

        if self.reference.is_some() {
            return match self.reference.as_ref() {
                Some(reference) => {
                    reference.resolve_ref(components, depth_tracker)
                }
                None => Ok(depth_tracker.empty_value()),
            };
        }

        match &self.schema {
            Some(schema) => serde_json::to_value(schema).map_err(|e| {
                execution_err(format!("Failed to resolve schema: {e}"))
            }),
            None => Ok(serde_json::Value::Null),
        }
    }
}
