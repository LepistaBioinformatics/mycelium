use crate::settings::SESSION_KEY_PREFIX;

use myc_config::env_or_value::EnvOrValue;
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
                "{}_{}_password_change_token",
                SESSION_KEY_PREFIX, session_key
            );
        }

        format!("{}_{}", SESSION_KEY_PREFIX, session_key)
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
    pub token_secret_key: EnvOrValue<String>,
    pub token_expiration: i64,
    pub token_hmac_secret: EnvOrValue<String>,
    pub token_email_notifier: String,
}
