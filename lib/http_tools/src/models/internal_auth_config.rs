use myc_config::secret_resolver::SecretResolver;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InternalOauthConfig {
    pub jwt_secret: SecretResolver<String>,
    pub jwt_expires_in: SecretResolver<i64>,
    pub tmp_expires_in: SecretResolver<i64>,
}
