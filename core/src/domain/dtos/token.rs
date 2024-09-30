use super::{email::Email, user::PasswordHash};

use chrono::{DateTime, Local};
use mycelium_base::utils::errors::{dto_err, MappedErrors};
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BaseMeta<T>
where
    T: ToString,
{
    /// This is the user id to which the email confirmation token is related
    pub user_id: Uuid,

    /// This is the email to which the email confirmation token is related
    pub email: Email,

    /// This is the email confirmation token
    token: T,
}

impl<T> BaseMeta<T>
where
    T: Clone + ToString + TryFrom<String>,
{
    pub(crate) fn new(user_id: Uuid, email: Email, token: T) -> Self {
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

        BaseMeta::<T>::new(user_id, email.to_owned(), token)
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

    pub fn check_password(&self, password: &[u8]) -> Result<(), MappedErrors> {
        PasswordHash::new_from_hash(self.token.to_string())
            .check_password(password)
    }

    pub(crate) fn get_token(&self) -> T {
        self.token.to_owned()
    }
}

pub type EmailConfirmationTokenMeta = BaseMeta<String>;

pub type PasswordChangeTokenMeta = BaseMeta<String>;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub enum TokenMeta {
    /// This is the email confirmation token
    EmailConfirmation(EmailConfirmationTokenMeta),

    /// This is the password change token
    PasswordChange(PasswordChangeTokenMeta),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Token {
    id: Option<i32>,
    pub expiration: DateTime<Local>,
    pub meta: TokenMeta,
}

impl Token {
    pub fn get_id(&self) -> Option<i32> {
        self.id
    }

    pub fn new(
        id: Option<i32>,
        expiration: DateTime<Local>,
        meta: TokenMeta,
    ) -> Self {
        Self {
            id,
            expiration,
            meta,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_encrypted_token_new() {
        let token = "123456".to_string();
        let mut passwd_token = PasswordChangeTokenMeta::new(
            Uuid::new_v4(),
            Email {
                username: "test".to_string(),
                domain: "test.com".to_string(),
            },
            token.to_owned(),
        );

        let encryption_res = passwd_token.encrypted_token();

        assert!(encryption_res.is_ok());
        assert!(passwd_token.check_password(token.as_bytes()).is_ok());
    }
}
