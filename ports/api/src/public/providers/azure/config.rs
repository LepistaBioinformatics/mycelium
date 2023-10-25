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
    pub jwt_secret: String,
    pub jwt_expires_in: String,
    pub jwt_max_age: i64,
    pub azure_oauth_client_id: String,
    pub azure_oauth_client_secret: String,
    pub azure_oauth_redirect_url: String,
}

impl AzureOauthConfig {
    /* pub fn init() -> AzureOauthConfig {
        let client_origin = std::env::var("AZURE_CLIENT_ORIGIN")
            .expect("AZURE_CLIENT_ORIGIN must be set");

        let jwt_secret = std::env::var("AZURE_JWT_SECRET")
            .expect("AZURE_JWT_SECRET must be set");

        let jwt_expires_in = std::env::var("AZURE_TOKEN_EXPIRED_IN")
            .expect("AZURE_TOKEN_EXPIRED_IN must be set");

        let jwt_max_age = std::env::var("AZURE_TOKEN_MAX_AGE")
            .expect("AZURE_TOKEN_MAX_AGE must be set");

        let azure_oauth_client_id = std::env::var("AZURE_OAUTH_CLIENT_ID")
            .expect("AZURE_OAUTH_CLIENT_ID must be set");

        let azure_oauth_client_secret =
            std::env::var("AZURE_OAUTH_CLIENT_SECRET")
                .expect("AZURE_OAUTH_CLIENT_SECRET must be set");

        let azure_oauth_redirect_url =
            std::env::var("AZURE_OAUTH_REDIRECT_URL")
                .expect("AZURE_OAUTH_REDIRECT_URL must be set");

        let azure_oauth_authorization_url =
            std::env::var("AZURE_OAUTH_AUTHORIZATION_URL")
                .expect("AZURE_OAUTH_AUTHORIZATION_URL must be set");

        let azure_oauth_token_url = std::env::var("AZURE_OAUTH_TOKEN_URL")
            .expect("AZURE_OAUTH_TOKEN_URL must be set");

        AzureOauthConfig {
            client_origin,
            jwt_secret,
            jwt_expires_in,
            jwt_max_age: jwt_max_age.parse::<i64>().unwrap(),
            azure_oauth_client_id,
            azure_oauth_client_secret,
            azure_oauth_redirect_url,
        }
    } */

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

        let client = BasicClient::new(
            ClientId::new(_config.azure_oauth_client_id.to_owned()),
            Some(ClientSecret::new(
                _config.azure_oauth_client_secret.to_owned(),
            )),
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

// * ---------------------------------------------------------------------------
// * TESTS
// * ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_authorization_code_generation() {
        let config = AzureOauthConfig {
            jwt_secret: "secret".to_string(),
            jwt_expires_in: "60m".to_string(),
            jwt_max_age: 60,
            client_origin: "http://localhost:8080".to_string(),
            azure_oauth_client_id: "2e1852b0-4d34-4418-9060-19aa148cb658"
                .to_string(),
            azure_oauth_client_secret:
                "NNr8Q~yemCzpvJcv2jaDloY91uZnC895EKmyybY~".to_string(),
            azure_oauth_redirect_url: "http://localhost:8080/myc/auth"
                .to_string(),
        };

        let result = config.init_basic_client(None);

        assert!(result.is_ok());
    }
}
