use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationDTO {
    pub id: Option<Uuid>,

    pub name: String,
    pub description: String,
}
