use myc_config::secret_resolver::SecretResolver;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AzureOauthConfig {
    pub csrf_token_expiration: SecretResolver<i64>,
    pub callback_path: SecretResolver<String>,
    pub redirect_url: SecretResolver<String>,
    pub jwt_secret: SecretResolver<String>,
    pub tenant_id: SecretResolver<String>,
    pub client_id: SecretResolver<String>,
    pub client_secret: SecretResolver<String>,
}
