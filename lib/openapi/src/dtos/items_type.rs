use crate::{
    dtos::schema::Schema,
    entities::{DepthTracker, ReferenceResolver},
};

use mycelium_base::utils::errors::MappedErrors;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq, ToSchema)]
#[serde(untagged, rename_all = "camelCase")]
pub enum ItemsType {
    #[schema(no_recursion)]
    Schema(Schema),
    Boolean(bool),
}

impl ReferenceResolver for ItemsType {
    fn resolve_ref(
        &self,
        components: &serde_json::Value,
        depth_tracker: &mut DepthTracker,
    ) -> Result<serde_json::Value, MappedErrors> {
        if depth_tracker.should_stop() {
            return Ok(depth_tracker.empty_value());
        }

        depth_tracker.increment();

        match self {
            ItemsType::Schema(schema) => {
                schema.resolve_ref(components, depth_tracker)
            }
            ItemsType::Boolean(boolean) => {
                Ok(serde_json::to_value(boolean).unwrap())
            }
        }
    }
}
