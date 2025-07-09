use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(
    Debug, Deserialize, Serialize, Copy, Clone, Eq, PartialEq, ToSchema,
)]
#[serde(rename_all = "camelCase")]
pub enum Location {
    Query,
    Path,
    Header,
    Cookie,
}
