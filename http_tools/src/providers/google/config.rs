use myc_config::env_or_value::EnvOrValue;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GoogleOauthConfig {
    pub client_origin: String,
    pub jwt_secret: EnvOrValue<String>,
    pub jwt_expires_in: String,
    pub jwt_max_age: i64,
    pub client_id: EnvOrValue<String>,
    pub client_secret: EnvOrValue<String>,
    pub redirect_url: String,
}
