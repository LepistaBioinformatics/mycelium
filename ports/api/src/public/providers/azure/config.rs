use myc_config::env_or_value::EnvOrValue;
///
/// Temporary disabled
///
/// TODO: Implement Azure OAuth2
///
use oauth2::{
    basic::BasicClient, reqwest::http_client, AuthUrl, AuthorizationCode,
    ClientId, ClientSecret, CsrfToken, PkceCodeChallenge, RedirectUrl, Scope,
    TokenResponse, TokenUrl,
};
use serde::{Deserialize, Serialize};
use url::ParseError;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AzureOauthConfig {
    pub client_origin: String,
    pub jwt_secret: EnvOrValue<String>,
    pub jwt_expires_in: String,
    pub jwt_max_age: i64,
    pub azure_oauth_client_id: String,
    pub azure_oauth_client_secret: EnvOrValue<String>,
    pub azure_oauth_redirect_url: String,
}

impl AzureOauthConfig {
    pub fn init_basic_client(
        &self,
        custom_config: Option<AzureOauthConfig>,
    ) -> Result<(), ParseError> {
        let mut _config = self.to_owned();

        if let Some(config) = custom_config {
            _config = config;
        }

        let authorization_url = AuthUrl::new(
            "https://login.microsoftonline.com/common/oauth2/v2.0/authorize"
                .to_string(),
        )?;

        let token_url = TokenUrl::new(
            "https://login.microsoftonline.com/common/oauth2/v2.0/token"
                .to_string(),
        )?;

        let secret = match _config.azure_oauth_client_secret.get() {
            Ok(secret) => secret,
            Err(err) => {
                panic!("Could not retrieve client secret: {err}");
            }
        };

        let client = BasicClient::new(
            ClientId::new(_config.azure_oauth_client_id.to_owned()),
            Some(ClientSecret::new(secret.to_owned())),
            authorization_url,
            Some(token_url),
        )
        // Set the URL the user will be redirected to after the authorization process.
        .set_redirect_uri(RedirectUrl::new(
            _config.azure_oauth_redirect_url.to_owned(),
        )?);

        // Generate a PKCE challenge.
        let (pkce_challenge, pkce_verifier) =
            PkceCodeChallenge::new_random_sha256();

        // Generate the full authorization URL.
        let (auth_url, csrf_token) = client
            .authorize_url(CsrfToken::new_random)
            // Set the desired scopes.
            .add_scope(Scope::new("email".to_string()))
            .add_scope(Scope::new("profile".to_string()))
            .add_scope(Scope::new("openid".to_string()))
            /* .add_scope(Scope::new(
                "https://www.googleapis.com/auth/userinfo.profile".to_string(),
            ))
            .add_scope(Scope::new(
                "https://www.googleapis.com/auth/userinfo.email".to_string(),
            )) */
            // Set the PKCE code challenge.
            .set_pkce_challenge(pkce_challenge)
            .url();

        println!("CSRF token: {:?}", csrf_token.secret());

        // This is the URL you should redirect the user to, in order to trigger
        // the authorization process.
        println!("Browse to: {}", auth_url);

        // Once the user has been redirected to the redirect URL, you'll have
        // access to the authorization code. For security reasons, your code
        // should verify that the `state` parameter returned by the server
        // matches `csrf_state`.

        // Now you can trade it for an access token.
        let token_result = match client
            .exchange_code(AuthorizationCode::new(
                "some_authorization_code".to_string(),
            ))
            // Set the PKCE code verifier.
            .set_pkce_verifier(pkce_verifier)
            .request(http_client)
        {
            Ok(token_result) => token_result,
            Err(err) => {
                println!("Failed to contact token endpoint: {err}");
                return Ok(());
            }
        };

        println!("Access token: {:?}", token_result.access_token());

        Ok(())
    }
}
