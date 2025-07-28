use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, Serialize, Eq, PartialEq, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct KVArtifact<T: Serialize + Deserialize<'static>> {
    pub key: String,
    pub value: T,
}
