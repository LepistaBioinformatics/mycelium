/// Service Account Related Metadata
///
/// This module contains the data transfer objects for metadata related to
/// service account tokens
///
use crate::{
    domain::dtos::{email::Email, user::PasswordHash},
    models::AccountLifeCycle,
};

use hmac::Hmac;
use mycelium_base::utils::errors::{dto_err, MappedErrors};
use serde::{Deserialize, Serialize};
use sha2::Sha512;
use uuid::Uuid;

pub(crate) type HmacSha256 = Hmac<Sha512>;

pub trait ScopedBehavior {
    fn sign_token(
        &mut self,
        config: AccountLifeCycle,
        extra_data: Option<String>,
    ) -> impl std::future::Future<Output = Result<String, MappedErrors>> + Send;
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ServiceAccountRelatedMeta<TokenType, Scope>
where
    TokenType: ToString,
    Scope: ToString,
{
    /// The scope which the token should be used
    ///
    /// Scopes should be defined by the service needing the token
    pub scope: Scope,

    /// This is the user id to which the token was issued
    pub user_id: Uuid,

    /// This is the email to which the token is related
    pub email: Email,

    /// This is the email confirmation token
    token: TokenType,
}

impl<TokenType, Scope> ServiceAccountRelatedMeta<TokenType, Scope>
where
    TokenType: Clone + ToString + TryFrom<String>,
    Scope: Clone + ToString + TryFrom<String> + ScopedBehavior,
{
    /// Create a new ServiceAccountRelatedMeta
    ///
    /// This function creates a new ServiceAccountRelatedMeta object
    fn new(
        scope: Scope,
        user_id: Uuid,
        email: Email,
        token: TokenType,
    ) -> Self {
        Self {
            scope,
            user_id,
            email,
            token,
        }
    }

    /// Create a new signed token
    ///
    /// This function creates a new signed token using the scope, the user_id
    /// and the email provided
    ///
    pub(crate) async fn new_signed_token(
        scope: &mut Scope,
        user_id: Uuid,
        email: Email,
        config: AccountLifeCycle,
    ) -> Result<Self, MappedErrors> {
        let extra_data = format!("{} <{}>", user_id, email.email());
        let signature = scope.sign_token(config, Some(extra_data)).await?;

        let token = match TokenType::try_from(signature) {
            Ok(token) => token,
            Err(_) => {
                return dto_err("Unexpected error on token processing")
                    .as_error()
            }
        };

        Ok(ServiceAccountRelatedMeta::<TokenType, Scope>::new(
            scope.to_owned(),
            user_id,
            email.to_owned(),
            token,
        ))
    }

    /// Encrypts the token
    ///
    /// Token encryption is done using the password hashing algorithm. This
    /// function is used to encrypt the token before storing it in the database
    ///
    pub fn encrypted_token(&mut self) -> Result<(), MappedErrors> {
        let hash = match PasswordHash::hash_user_password(
            self.token.to_string().as_bytes(),
        )
        .hash
        .try_into()
        {
            Ok(hash) => hash,
            Err(_) => {
                return dto_err("Unexpected error on token processing")
                    .as_error()
            }
        };

        self.token = hash;

        Ok(())
    }

    /// Check the token
    ///
    /// This function checks the token against the token provided
    pub fn check_token(&self, token: &[u8]) -> Result<(), MappedErrors> {
        PasswordHash::new_from_hash(self.token.to_string())
            .check_password(token)
    }
}
