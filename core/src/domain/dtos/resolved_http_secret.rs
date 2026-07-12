use crate::domain::dtos::http_secret::HttpSecret;

use mycelium_base::utils::errors::MappedErrors;

/// An `HttpSecret` with its `token` resolved to a plain string.
///
/// `HttpSecret.token` is a `SecretResolver<String>`, which may need an async
/// env/vault lookup to reach a usable value. This type carries the resolved
/// value so it can be read synchronously when building outgoing requests.
///
pub enum ResolvedHttpSecret {
    AuthorizationHeader {
        header_name: Option<String>,
        prefix: Option<String>,
        token: String,
    },
    QueryParameter {
        name: String,
        token: String,
    },
}

impl ResolvedHttpSecret {
    #[tracing::instrument(name = "resolve_http_secret", skip_all)]
    pub async fn from_http_secret(
        secret: HttpSecret,
    ) -> Result<Self, MappedErrors> {
        Ok(match secret {
            HttpSecret::AuthorizationHeader {
                header_name,
                prefix,
                token,
            } => Self::AuthorizationHeader {
                header_name,
                prefix,
                token: token.async_get_or_error().await?,
            },
            HttpSecret::QueryParameter { name, token } => {
                Self::QueryParameter {
                    name,
                    token: token.async_get_or_error().await?,
                }
            }
        })
    }
}
