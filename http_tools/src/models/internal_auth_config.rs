use myc_config::env_or_value::EnvOrValue;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InternalOauthConfig {
    pub jwt_secret: EnvOrValue<String>,
    pub jwt_expires_in: i64,
    pub tmp_expires_in: i64,
}
