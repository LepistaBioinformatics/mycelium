use myc_config::env_or_value::EnvOrValue;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AzureOauthConfig {
    pub client_origin: String,
    pub jwt_secret: EnvOrValue<String>,
    pub csrf_token_expiration: i64,
    pub tenant_id: String,
    pub client_id: String,
    pub client_secret: EnvOrValue<String>,
    pub redirect_url: String,
    pub callback_path: Option<String>,
}
