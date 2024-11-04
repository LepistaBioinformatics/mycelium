/// User Related Metadata
///
/// This module contains the data transfer objects for metadata related to user
/// operations
///
use crate::domain::dtos::{email::Email, user::PasswordHash};

use mycelium_base::utils::errors::{dto_err, MappedErrors};
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserRelatedMeta<TokenType>
where
    TokenType: ToString,
{
    /// This is the user id to which the email confirmation token is related
    pub user_id: Uuid,

    /// This is the email to which the email confirmation token is related
    pub email: Email,

    /// This is the email confirmation token
    token: TokenType,
}

impl<TokenType> UserRelatedMeta<TokenType>
where
    TokenType: Clone + ToString + TryFrom<String>,
{
    pub(crate) fn new(user_id: Uuid, email: Email, token: TokenType) -> Self {
        Self {
            user_id,
            email,
            token,
        }
    }

    pub(crate) fn new_with_random_token(
        user_id: Uuid,
        email: Email,
        min: u32,
        max: u32,
    ) -> Self {
        let random_number = thread_rng().gen_range(min..max);
        let fixed_size_string = format!("{:06}", random_number);

        let token = match fixed_size_string.try_into() {
            Ok(token) => token,
            Err(_) => panic!("Error converting random number to token"),
        };

        UserRelatedMeta::<TokenType>::new(user_id, email.to_owned(), token)
    }

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

    pub(crate) fn get_token(&self) -> TokenType {
        self.token.to_owned()
    }
}
