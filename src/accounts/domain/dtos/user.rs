use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UserDTO {
    pub id: String,

    pub username: String,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
}
