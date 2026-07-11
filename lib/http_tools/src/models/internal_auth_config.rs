use myc_config::secret_resolver::SecretResolver;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InternalOauthConfig {
    pub jwt_secret: SecretResolver<String>,
    #[serde(default = "default_jwt_expires_in")]
    pub jwt_expires_in: SecretResolver<i64>,
    #[serde(default = "default_tmp_expires_in")]
    pub tmp_expires_in: SecretResolver<i64>,
}

fn default_jwt_expires_in() -> SecretResolver<i64> {
    SecretResolver::Value(43200)
}

fn default_tmp_expires_in() -> SecretResolver<i64> {
    SecretResolver::Value(300)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn internal_oauth_config_defaults_when_expiry_fields_absent() {
        let toml = r#"jwtSecret = "placeholder""#;
        let config: InternalOauthConfig = toml::from_str(toml).unwrap();

        assert_eq!(config.jwt_expires_in, SecretResolver::Value(43200));
        assert_eq!(config.tmp_expires_in, SecretResolver::Value(300));
    }
}
