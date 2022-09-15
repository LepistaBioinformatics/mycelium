use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ApplicationDTO {
    pub id: String,

    pub name: String,
    pub description: String,
}
