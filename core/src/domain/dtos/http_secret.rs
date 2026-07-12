use crate::{
    domain::utils::{decrypt_string_with_dek, encrypt_with_dek},
    models::AccountLifeCycle,
};

use myc_config::secret_resolver::SecretResolver;
use mycelium_base::utils::errors::{dto_err, MappedErrors};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use utoipa::ToSchema;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum HttpSecret {
    /// Authentication header
    ///
    /// The secret is passed as an authentication header.
    ///
    #[serde(rename_all = "camelCase")]
    AuthorizationHeader {
        /// The header name
        ///
        /// The name of the header. For example, if the name is `Authorization`,
        /// the header will be `Authorization Bear: <token value>`. The default
        /// value is `Authorization`.
        ///
        #[serde(default = "default_authorization_key")]
        header_name: Option<String>,

        /// The header prefix
        ///
        /// If present the prefix is added to the header. For example, if the
        /// prefix is `Bearer`, the header will be `Authorization Bearer: <token
        /// value>`.
        ///
        prefix: Option<String>,

        /// The header token
        ///
        /// The token is the value of the header. For example, if the token is
        /// `1234`, the header will be `Authorization Bearer: 123
        ///
        token: SecretResolver<String>,
    },

    #[serde(rename_all = "camelCase")]
    QueryParameter {
        /// The query parameter name
        ///
        /// The name of the query parameter. For example, if the name is `token`,
        /// the query parameter will be `?token=<token value>`.
        ///
        name: String,

        /// The query parameter value
        ///
        /// The value of the query parameter. For example, if the value is `1234`,
        /// the query parameter will be `?token=1234`.
        ///
        token: SecretResolver<String>,
    },
}

pub fn default_authorization_key() -> Option<String> {
    Some("Authorization".to_string())
}

impl HttpSecret {
    /// Encrypt the token with the system DEK (v2 format).
    ///
    /// Only `SecretResolver::Value` tokens hold a plaintext secret at rest —
    /// `Env`/`Vault` tokens are resolved externally on each use and pass
    /// through untouched.
    #[tracing::instrument(name = "encrypt_me", skip_all)]
    pub(crate) fn encrypt_me(
        &self,
        dek: &[u8; 32],
        aad: &[u8],
    ) -> Result<Self, MappedErrors> {
        let token = match self {
            Self::AuthorizationHeader { token, .. } => token,
            Self::QueryParameter { token, .. } => token,
        };

        let SecretResolver::Value(plain_token) = token else {
            return Ok(self.to_owned());
        };

        let encrypted_string = encrypt_with_dek(plain_token, dek, aad)?;
        let token = SecretResolver::Value(encrypted_string);

        Ok(match self {
            Self::AuthorizationHeader {
                header_name,
                prefix,
                ..
            } => Self::AuthorizationHeader {
                token,
                header_name: header_name.to_owned(),
                prefix: prefix.to_owned(),
            },
            Self::QueryParameter { name, .. } => Self::QueryParameter {
                token,
                name: name.to_owned(),
            },
        })
    }

    /// Decrypt the token.
    ///
    /// Detects v1 (no prefix) or v2 (`v2:` prefix) automatically. v2 uses the
    /// supplied `dek`; v1 falls back to the legacy KEK path via `config`.
    ///
    /// The v1 fallback derives the KEK directly from
    /// `AccountLifeCycle::token_secret`. This ties any webhook secret still
    /// in v1 format to the current `token_secret` value — rotation of
    /// `token_secret` before running `migrate-dek` will make those
    /// ciphertexts unreadable. See
    /// `AccountLifeCycle::derive_kek_bytes` for the full list of
    /// `token_secret` consumers and rotation caveats.
    #[tracing::instrument(name = "decrypt_me", skip_all)]
    pub(crate) async fn decrypt_me(
        &self,
        dek: &[u8; 32],
        config: &AccountLifeCycle,
        aad: &[u8],
    ) -> Result<Self, MappedErrors> {
        let token = match self {
            Self::AuthorizationHeader { token, .. } => token,
            Self::QueryParameter { token, .. } => token,
        };

        let SecretResolver::Value(encrypted_token) = token else {
            return Ok(self.to_owned());
        };

        let decrypted_secret =
            decrypt_string_with_dek(encrypted_token, config, dek, aad).await?;
        let token = SecretResolver::Value(decrypted_secret);

        Ok(match self {
            Self::AuthorizationHeader {
                header_name,
                prefix,
                ..
            } => Self::AuthorizationHeader {
                token,
                header_name: header_name.to_owned(),
                prefix: prefix.to_owned(),
            },
            Self::QueryParameter { name, .. } => Self::QueryParameter {
                token,
                name: name.to_owned(),
            },
        })
    }

    #[tracing::instrument(name = "redact_token", skip_all)]
    pub(crate) fn redact_token(&mut self) {
        let token = match self {
            Self::AuthorizationHeader { token, .. } => token,
            Self::QueryParameter { token, .. } => token,
        };

        let SecretResolver::Value(_) = token else {
            return;
        };

        *token = SecretResolver::Value("REDACTED".to_string());
    }
}

impl FromStr for HttpSecret {
    type Err = MappedErrors;

    /// Parse the secret from a string
    ///
    /// Try to parse from JSON and YAML. If none of them work, return an error.
    ///
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let json_try = serde_json::from_str::<HttpSecret>(s);
        let toml_try = toml::from_str::<HttpSecret>(s);

        if let Ok(secret) = json_try {
            return Ok(secret);
        }

        if let Ok(secret) = toml_try {
            return Ok(secret);
        }

        dto_err("Failed to parse secret").as_error()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_authorization_header_token_env_variant_parses() {
        let toml = r#"
            authorizationHeader = { headerName = "Authorization", prefix = "Bearer", token = { env = "MY_TOKEN_VAR" } }
        "#;

        let secret: HttpSecret = toml::from_str(toml).unwrap();

        match secret {
            HttpSecret::AuthorizationHeader { token, .. } => {
                assert_eq!(token, SecretResolver::Env("MY_TOKEN_VAR".into()));
            }
            other => panic!("Expected AuthorizationHeader, got {other:?}"),
        }
    }

    #[test]
    fn test_query_parameter_token_vault_variant_parses() {
        let toml = r#"
            queryParameter = { name = "token", token = { vault = { path = "myc/services/api", key = "token" } } }
        "#;

        let secret: HttpSecret = toml::from_str(toml).unwrap();

        match secret {
            HttpSecret::QueryParameter { token, .. } => {
                assert_eq!(
                    token,
                    SecretResolver::Vault {
                        path: "myc/services/api".into(),
                        key: "token".into(),
                    }
                );
            }
            other => panic!("Expected QueryParameter, got {other:?}"),
        }
    }

    #[test]
    fn test_authorization_header_token_literal_parses_as_value() {
        let toml = r#"
            authorizationHeader = { headerName = "Authorization", prefix = "Bearer", token = "literal-token" }
        "#;

        let secret: HttpSecret = toml::from_str(toml).unwrap();

        match secret {
            HttpSecret::AuthorizationHeader { token, .. } => {
                assert_eq!(
                    token,
                    SecretResolver::Value("literal-token".to_string())
                );
            }
            other => panic!("Expected AuthorizationHeader, got {other:?}"),
        }
    }

    #[test]
    fn test_encrypt_me_passes_through_env_token_untouched() {
        let secret = HttpSecret::AuthorizationHeader {
            header_name: None,
            prefix: None,
            token: SecretResolver::Env("MY_TOKEN_VAR".to_string()),
        };

        let dek = [0u8; 32];
        let aad = b"aad";
        let encrypted = secret.encrypt_me(&dek, aad).unwrap();

        assert_eq!(encrypted, secret);
    }

    #[test]
    fn test_redact_token_ignores_env_token() {
        let mut secret = HttpSecret::QueryParameter {
            name: "token".to_string(),
            token: SecretResolver::Env("MY_TOKEN_VAR".to_string()),
        };

        secret.redact_token();

        match secret {
            HttpSecret::QueryParameter { token, .. } => {
                assert_eq!(token, SecretResolver::Env("MY_TOKEN_VAR".into()));
            }
            other => panic!("Expected QueryParameter, got {other:?}"),
        }
    }
}
