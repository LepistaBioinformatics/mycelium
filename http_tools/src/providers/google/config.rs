use myc_config::secret_resolver::SecretResolver;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GoogleOauthConfig {
    pub client_origin: String,
    pub jwt_secret: SecretResolver<String>,
    pub jwt_expires_in: String,
    pub jwt_max_age: i64,
    pub client_id: SecretResolver<String>,
    pub client_secret: SecretResolver<String>,
    pub redirect_url: String,
}
