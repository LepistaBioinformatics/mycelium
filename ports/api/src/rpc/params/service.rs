use schemars::JsonSchema;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Default, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListDiscoverableServicesParams {
    pub id: Option<Uuid>,
    pub name: Option<String>,
}
