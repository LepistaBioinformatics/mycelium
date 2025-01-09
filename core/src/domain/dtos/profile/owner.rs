use crate::domain::dtos::user::User;

use mycelium_base::utils::errors::{dto_err, MappedErrors};
use serde::{Deserialize, Serialize};
use utoipa::{ToResponse, ToSchema};
use uuid::Uuid;

#[derive(
    Clone, Debug, Deserialize, Serialize, ToSchema, Eq, PartialEq, ToResponse,
)]
#[serde(rename_all = "camelCase")]
pub struct Owner {
    pub id: Uuid,

    /// The owner email
    ///
    /// The email of the user that administrate the profile. Email denotes the
    /// central part of the profile management. Email should be used to collect
    /// licensed IDs and perform guest operations. Thus, it should be unique in
    /// the Mycelium platform.
    pub email: String,

    /// The owner first name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_name: Option<String>,

    /// The owner last name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_name: Option<String>,

    /// The owner username
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,

    /// If the owner is the principal account owner
    pub is_principal: bool,
}

impl Owner {
    pub fn from_user(user: User) -> Result<Self, MappedErrors> {
        let user_id = match user.id {
            Some(id) => id,
            None => {
                return dto_err("User ID should not be empty".to_string())
                    .as_error()
            }
        };

        Ok(Self {
            id: user_id,
            email: user.email.email(),
            first_name: user.to_owned().first_name,
            last_name: user.to_owned().last_name,
            username: Some(user.to_owned().username),
            is_principal: user.is_principal(),
        })
    }
}
