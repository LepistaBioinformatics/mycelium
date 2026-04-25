use crate::{
    domain::utils::{decrypt_string_with_dek, encrypt_with_dek},
    models::AccountLifeCycle,
};

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
        token: String,
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
        token: String,
    },
}

pub fn default_authorization_key() -> Option<String> {
    Some("Authorization".to_string())
}

impl HttpSecret {
    /// Encrypt the token with the system DEK (v2 format).
    #[tracing::instrument(name = "encrypt_me", skip_all)]
    pub(crate) fn encrypt_me(
        &self,
        dek: &[u8; 32],
        aad: &[u8],
    ) -> Result<Self, MappedErrors> {
        let plain_token = match self {
            Self::AuthorizationHeader { token, .. } => token.as_str(),
            Self::QueryParameter { token, .. } => token.as_str(),
        };

        let encrypted_string = encrypt_with_dek(plain_token, dek, aad)?;

        Ok(match self {
            Self::AuthorizationHeader {
                header_name,
                prefix,
                ..
            } => Self::AuthorizationHeader {
                token: encrypted_string,
                header_name: header_name.to_owned(),
                prefix: prefix.to_owned(),
            },
            Self::QueryParameter { name, .. } => Self::QueryParameter {
                token: encrypted_string,
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
            Self::AuthorizationHeader { token, .. } => token.as_str(),
            Self::QueryParameter { token, .. } => token.as_str(),
        };

        let decrypted_secret =
            decrypt_string_with_dek(token, config, dek, aad).await?;

        Ok(match self {
            Self::AuthorizationHeader {
                header_name,
                prefix,
                ..
            } => Self::AuthorizationHeader {
                token: decrypted_secret,
                header_name: header_name.to_owned(),
                prefix: prefix.to_owned(),
            },
            Self::QueryParameter { name, .. } => Self::QueryParameter {
                token: decrypted_secret,
                name: name.to_owned(),
            },
        })
    }

    #[tracing::instrument(name = "redact_token", skip_all)]
    pub(crate) fn redact_token(&mut self) {
        let redacted_word = "REDACTED".to_string();

        match self {
            Self::AuthorizationHeader { token, .. } => {
                *token = redacted_word;
            }
            Self::QueryParameter { token, .. } => {
                *token = redacted_word;
            }
        }
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
