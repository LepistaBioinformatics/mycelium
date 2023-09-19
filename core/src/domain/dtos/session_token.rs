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

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TokenSettings {
    pub secret: TokenSecret,

    //
    // ! This is not a domain logic. Don't forget to move it to the smtp
    // adapter.
    //
    pub email: NotificationSettings,
    pub frontend_url: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TokenSecret {
    pub secret_key: String,
    pub token_expiration: i64,
    pub hmac_secret: String,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct NotificationSettings {
    pub host: String,
    pub host_user: String,
    pub host_user_password: String,
}
