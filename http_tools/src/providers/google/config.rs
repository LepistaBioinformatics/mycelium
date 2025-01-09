use myc_config::secret_resolver::SecretResolver;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GoogleOauthConfig {
    pub client_origin: SecretResolver<String>,
    pub jwt_max_age: SecretResolver<i64>,
    pub redirect_url: SecretResolver<String>,
    pub jwt_secret: SecretResolver<String>,
    pub client_id: SecretResolver<String>,
    pub client_secret: SecretResolver<String>,
}
