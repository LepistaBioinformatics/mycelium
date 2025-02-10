use myc_config::secret_resolver::SecretResolver;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalProviderConfig {
    /// The issuer of the token
    ///
    /// This is the issuer of the token that will be used to validate the token.
    ///
    /// Example:
    ///  - Google: https://accounts.google.com
    ///  - Microsoft: https://login.microsoftonline.com/{tenant_id}/v2.0
    ///  - Auth0: https://your-tenant.auth0.com/
    ///
    pub issuer: SecretResolver<String>,
    /// The jwks uri of the token
    ///
    /// This is the jwks uri of the token that will be used to validate the
    /// token.
    ///
    /// Example:
    ///  - Google: https://www.googleapis.com/oauth2/v1/certs
    ///  - Microsoft: https://login.microsoftonline.com/{tenant_id}/discovery/v2.0/keys
    ///  - Auth0: https://{your-auth0-domain}/.well-known/jwks.json
    ///
    pub jwks_uri: SecretResolver<String>,
    /// The audience of the token
    ///
    /// This is the audience of the token that will be used to validate the
    /// token.
    ///
    /// Example:
    ///  - Google: YOUR_CLIENT_ID
    ///  - Microsoft: YOUR_CLIENT_ID
    ///  - Auth0: YOUR_CLIENT_ID
    ///
    pub audience: SecretResolver<String>,

    /// The user info url of the token
    ///
    /// This is the user info url of the token that will be used to validate the
    /// token.
    ///
    /// Example:
    ///  - Google: https://www.googleapis.com/oauth2/v3/userinfo
    ///  - Microsoft: https://graph.microsoft.com/oidc/userinfo
    ///  - Auth0: https://{your-auth0-domain}.auth0.com/userinfo
    ///
    pub user_info_url: Option<SecretResolver<String>>,
}
