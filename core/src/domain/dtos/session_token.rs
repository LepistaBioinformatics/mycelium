use crate::settings::SESSION_KEY_PREFIX;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SessionToken {
    pub user_id: Uuid,
}

impl SessionToken {
    pub fn build_prefixed_session_token(
        session_key: String,
        for_password_change: Option<bool>,
    ) -> String {
        if let Some(true) = for_password_change {
            return format!(
                "{}{}_password_change_token",
                SESSION_KEY_PREFIX, session_key
            );
        }

        format!("{}{}", SESSION_KEY_PREFIX, session_key)
    }
}

/// This struct is used to manage the token secret and the token expiration
/// times.
///
/// This is not the final position of this struct, it will be moved to a
/// dedicated module in the future.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TokenSecret {
    pub secret_key: String,
    pub token_expiration: i64,
    pub hmac_secret: String,
}
