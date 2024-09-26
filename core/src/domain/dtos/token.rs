use super::email::Email;

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BaseMeta<T> {
    /// This is the user id to which the email confirmation token is related
    pub user_id: Uuid,

    /// This is the email to which the email confirmation token is related
    pub email: Email,

    /// This is the email confirmation token
    token: T,
}

pub type EmailConfirmationTokenMeta = BaseMeta<String>;

impl EmailConfirmationTokenMeta {
    pub fn new(user_id: Uuid, email: Email, token: String) -> Self {
        Self {
            user_id,
            email,
            token,
        }
    }

    pub fn token_match(&self, token: String) -> bool {
        self.token == token
    }

    pub fn get_token(&self) -> String {
        self.token.to_owned()
    }
}

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
