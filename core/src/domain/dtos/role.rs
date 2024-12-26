use serde::{Deserialize, Serialize};
use slugify::slugify;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Role {
    pub id: Option<Uuid>,

    pub name: String,
    pub slug: String,
    pub description: String,
}

impl Role {
    pub fn new(id: Option<Uuid>, name: String, description: String) -> Self {
        Self {
            id,
            name: name.to_owned(),
            slug: slugify!(&name),
            description,
        }
    }
}
