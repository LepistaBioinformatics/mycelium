use super::{account::Account, email::Email};

use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, PasswordHasher,
};
use chrono::{DateTime, Local};
use clean_base::{dtos::Parent, utils::errors::MappedErrors};
use serde::{ser::SerializeStruct, Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PasswordHash {
    pub hash: String,
    pub salt: String,
}

impl PasswordHash {
    pub fn new(hash: String, salt: String) -> Self {
        Self { hash, salt }
    }

    pub fn hash_user_password(password: &[u8]) -> Self {
        let salt = SaltString::generate(&mut OsRng);

        Self {
            salt: salt.to_string(),
            hash: Argon2::default()
                .hash_password(password, &salt)
                .expect("Unable to hash password.")
                .to_string(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum Provider {
    External,
    Internal(PasswordHash),
}

#[derive(Clone, Debug, Deserialize, PartialEq, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: Option<Uuid>,

    pub username: String,
    pub email: Email,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub is_active: bool,
    pub created: DateTime<Local>,
    pub updated: Option<DateTime<Local>>,
    pub account: Option<Parent<Account, Uuid>>,

    /// The user provider.
    ///
    /// Provider is a optional field but it should be None only during the
    /// collection of the user data from database. Such None initialization
    /// prevents that password hashes and salts should be exposed to the
    /// outside.
    ///
    /// ! Thus, be careful on change this field.
    ///
    pub provider: Option<Provider>,
}

impl Serialize for User {
    /// This method is required to avoid that the password hash and salt are
    /// exposed to the outside.
    ///
    /// These implementation force users which desires to recovery such data to
    /// actively use the `get_provider` method.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ::serde::ser::Serializer,
    {
        let mut user = self.clone();
        user.provider = None;

        let mut state = serializer.serialize_struct("User", 10)?;
        state.serialize_field("id", &user.id)?;
        state.serialize_field("username", &user.username)?;
        state.serialize_field("email", &user.email)?;
        state.serialize_field("firstName", &user.first_name)?;
        state.serialize_field("lastName", &user.last_name)?;
        state.serialize_field("isActive", &user.is_active)?;
        state.serialize_field("created", &user.created)?;
        state.serialize_field("updated", &user.updated)?;
        state.serialize_field("account", &user.account)?;
        state.serialize_field("provider", &user.provider)?;
        state.end()
    }
}

impl User {
    pub fn new_with_provider(
        username: Option<String>,
        email: Email,
        provider: Provider,
        first_name: Option<String>,
        last_name: Option<String>,
    ) -> Result<Self, MappedErrors> {
        Ok(Self {
            id: None,
            username: match username {
                Some(username) => username,
                None => email.to_owned().username,
            },
            email,
            first_name,
            last_name,
            provider: Some(provider),
            is_active: true,
            created: Local::now(),
            updated: None,
            account: None,
        })
    }

    pub fn get_provider(&self) -> Option<&Provider> {
        self.provider.as_ref()
    }
}
