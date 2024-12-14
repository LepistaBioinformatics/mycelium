use super::{config::AzureOauthConfig, models::MsGraphDecode};
use crate::providers::shared::check_token_online;

use actix_web::{http::header::Header, HttpRequest};
use actix_web_httpauth::headers::authorization::{Authorization, Bearer};
use myc_core::domain::dtos::email::Email;
use mycelium_base::utils::errors::{execution_err, MappedErrors};
use oauth2::{
    basic::BasicClient, AuthUrl, ClientId, ClientSecret, RedirectUrl, TokenUrl,
};

/// Try to collect the user email.
///
/// The real implementation should try to collect the user credentials from the
/// request and return the user email as response.
pub async fn check_credentials(
    req: HttpRequest,
) -> Result<Email, MappedErrors> {
    let auth = match Authorization::<Bearer>::parse(&req) {
        Err(err) => {
            return execution_err(format!("Invalid client request: {err}"))
                .as_error();
        }
        Ok(res) => res,
    };

    decode_bearer_token_on_ms_graph(auth.to_owned()).await
}

/// Decode the bearer token on MS Graph.
///
/// This function is used to decode the bearer token on MS Graph.
/// The real implementation should try to collect the user credentials from the
/// request and return the user email as response.
///
async fn decode_bearer_token_on_ms_graph(
    auth: Authorization<Bearer>,
) -> Result<Email, MappedErrors> {
    let token = auth.into_scheme().token().to_string();
    let token_response: MsGraphDecode =
        check_token_online(token, "https://graph.microsoft.com/v1.0/me/", None)
            .await?;

    Email::from_string(token_response.mail)
}

pub(super) fn oauth_client(
    auth_config: AzureOauthConfig,
) -> Result<BasicClient, MappedErrors> {
    let tenant = auth_config.tenant_id;

    let client_id = ClientId::new(auth_config.client_id);

    let client_secret =
        ClientSecret::new(auth_config.client_secret.get_or_error()?);

    let auth_url = match AuthUrl::new(
        format!(
            "https://login.microsoftonline.com/{tenant}/oauth2/v2.0/authorize"
        )
        .to_string(),
    ) {
        Ok(url) => url,
        Err(err) => {
            return execution_err(format!(
                "Invalid authorization endpoint URL: {err}"
            ))
            .as_error();
        }
    };

    let token_url = match TokenUrl::new(
        format!("https://login.microsoftonline.com/{tenant}/oauth2/v2.0/token")
            .to_string(),
    ) {
        Ok(url) => url,
        Err(err) => {
            return execution_err(format!("Invalid token endpoint URL: {err}"))
                .as_error();
        }
    };

    let redirect_url = match RedirectUrl::new(format!(
        "{redirect_url}{callback_path}",
        redirect_url = auth_config.redirect_url,
        callback_path = auth_config.callback_path
    )) {
        Ok(url) => url,
        Err(err) => {
            return execution_err(format!("Invalid redirect URL: {err}"))
                .as_error();
        }
    };

    let client = BasicClient::new(
        client_id,
        Some(client_secret),
        auth_url,
        Some(token_url),
    )
    .set_redirect_uri(redirect_url);

    Ok(client)
}
