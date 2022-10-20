use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserDTO {
    pub id: Option<Uuid>,

    pub username: String,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
}
