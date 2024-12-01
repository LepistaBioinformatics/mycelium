mod connection_string;
mod meta;
mod token;

pub use connection_string::*;
pub use meta::*;
pub use token::*;

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

// ? ---------------------------------------------------------------------------
// ? MultiTypeToken
//
// Data struct used to store different types of tokens
//
// ? ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub enum MultiTypeMeta {
    /// This is the email confirmation token
    EmailConfirmation(EmailConfirmationTokenMeta),

    /// This is the password change token
    PasswordChange(PasswordChangeTokenMeta),

    /// This is the account token
    AccountScopedConnectionString(AccountScopedConnectionString),

    /// This is the role token
    RoleScopedConnectionString(RoleScopedConnectionString),

    /// This is the token scoped connection string
    TenantScopedConnectionString(TenantScopedConnectionString),
}

// ? ---------------------------------------------------------------------------
// ? Token
//
// Data struct used to store a token
//
// ? ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Token {
    id: Option<i32>,
    pub expiration: DateTime<Local>,
    pub meta: MultiTypeMeta,
}

impl Token {
    pub fn get_id(&self) -> Option<i32> {
        self.id
    }

    pub fn new(
        id: Option<i32>,
        expiration: DateTime<Local>,
        meta: MultiTypeMeta,
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
    use crate::domain::dtos::email::Email;

    use uuid::Uuid;

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
    }
}
