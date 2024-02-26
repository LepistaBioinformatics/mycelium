use super::{account::Account, email::Email};

use argon2::{
    password_hash::{
        rand_core::OsRng, PasswordHash as Argon2PasswordHash, SaltString,
    },
    Argon2, PasswordHasher, PasswordVerifier,
};
use chrono::{DateTime, Local};
use mycelium_base::{
    dtos::Parent,
    utils::errors::{use_case_err, MappedErrors},
};
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

    pub fn check_password(&self, password: &[u8]) -> Result<(), MappedErrors> {
        let parsed_hash = match Argon2PasswordHash::new(&self.hash) {
            Ok(hash) => hash,
            Err(err) => {
                return use_case_err(format!(
                    "Unable to parse password hash: {err}",
                ))
                .as_error()
            }
        };

        match Argon2::default().verify_password(password, &parsed_hash) {
            Ok(_) => Ok(()),
            Err(err) => {
                use_case_err(format!("Unable to verify password: {err}"))
                    .as_error()
            }
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum Provider {
    External(String),
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

    /// If the user is the principal user of the account.
    ///
    /// The principal user contains information of the first email that created
    /// the account. This information is used to send emails to the principal
    /// user.
    ///
    /// Principal users should not be deleted or deactivated if the account has
    /// other users connected.
    ///
    is_principal: bool,

    /// The user provider.
    ///
    /// Provider is a optional field but it should be None only during the
    /// collection of the user data from database. Such None initialization
    /// prevents that password hashes and salts should be exposed to the
    /// outside.
    ///
    /// ! Thus, be careful on change this field.
    ///
    provider: Option<Provider>,
}

impl Serialize for User {
    /// This method is required to avoid that the password hash and salt are
    /// exposed to the outside.
    ///
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
        state.serialize_field("isPrincipal", &user.is_principal)?;
        state.serialize_field("created", &user.created)?;
        state.serialize_field("updated", &user.updated)?;
        state.serialize_field("account", &user.account)?;
        state.serialize_field("provider", &user.provider)?;
        state.end()
    }
}

impl User {
    // ? -----------------------------------------------------------------------
    // ? Constructors
    // ? -----------------------------------------------------------------------

    fn new_with_provider(
        username: Option<String>,
        email: Email,
        provider: Provider,
        first_name: Option<String>,
        last_name: Option<String>,
        is_principal: bool,
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
            is_principal,
            created: Local::now(),
            updated: None,
            account: None,
        })
    }

    pub fn new_principal_with_provider(
        username: Option<String>,
        email: Email,
        provider: Provider,
        first_name: Option<String>,
        last_name: Option<String>,
    ) -> Result<Self, MappedErrors> {
        Self::new_with_provider(
            username, email, provider, first_name, last_name, true,
        )
    }

    pub fn new_secondary_with_provider(
        username: Option<String>,
        email: Email,
        provider: Provider,
        first_name: Option<String>,
        last_name: Option<String>,
    ) -> Result<Self, MappedErrors> {
        Self::new_with_provider(
            username, email, provider, first_name, last_name, false,
        )
    }

    pub fn new(
        id: Option<Uuid>,
        username: String,
        email: Email,
        first_name: Option<String>,
        last_name: Option<String>,
        is_active: bool,
        created: DateTime<Local>,
        updated: Option<DateTime<Local>>,
        account: Option<Parent<Account, Uuid>>,
        provider: Option<Provider>,
    ) -> Self {
        Self {
            id,
            username,
            email,
            first_name,
            last_name,
            is_active,
            created,
            updated,
            account,
            provider,
            is_principal: false,
        }
    }

    // ? -----------------------------------------------------------------------
    // ? Instance methods
    // ? -----------------------------------------------------------------------

    pub fn with_principal(&mut self, is_principal: bool) -> Self {
        self.is_principal = is_principal.to_owned();
        self.to_owned()
    }

    pub fn is_principal(&self) -> bool {
        self.is_principal
    }

    pub fn provider(&self) -> Option<Provider> {
        self.provider.to_owned()
    }

    pub fn is_internal_or_error(&self) -> Result<bool, MappedErrors> {
        match self.provider {
            Some(Provider::Internal(_)) => Ok(true),
            Some(Provider::External(_)) => Ok(false),
            None => use_case_err(
                "User is probably registered but mycelium is unable to 
check if user is internal or not. The user provider is None.",
            )
            .as_error(),
        }
    }
}
