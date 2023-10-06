/// Temporary disabled
///
/// TODO: Implement Azure OAuth2
///
use super::config::Config;

use actix_web::{
    post,
    web::{self, Form},
    HttpResponse, Responder,
};
use oauth2::{
    basic::BasicClient, reqwest::async_http_client, AuthUrl, AuthorizationCode,
    ClientId, ClientSecret, PkceCodeVerifier, RedirectUrl, TokenUrl,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct TokenBody {
    grant_type: String,
    code: String,
    code_verifier: String,
    redirect_uri: String,
}

#[post("/token")]
async fn token(body: Form<TokenBody>) -> impl Responder {
    let req = body.into_inner();

    let _config = Config {
        jwt_secret: "secret".to_string(),
        jwt_expires_in: "60m".to_string(),
        jwt_max_age: 60,
        client_origin: "http://localhost:8080".to_string(),
        azure_oauth_client_id: "2e1852b0-4d34-4418-9060-19aa148cb658"
            .to_string(),
        azure_oauth_client_secret: "NNr8Q~yemCzpvJcv2jaDloY91uZnC895EKmyybY~"
            .to_string(),
        azure_oauth_redirect_url: "http://localhost:8080/myc/auth".to_string(),
    };

    let authorization_url = match AuthUrl::new(
        "https://login.microsoftonline.com/common/oauth2/v2.0/authorize"
            .to_string(),
    ) {
        Ok(url) => url,
        Err(err) => return HttpResponse::InternalServerError().json(
            serde_json::json!({
                "status": "error",
                "message": format!("Failed to parse authorization URL: {err}")
            }),
        ),
    };

    let token_url = match TokenUrl::new(
        "https://login.microsoftonline.com/common/oauth2/v2.0/token"
            .to_string(),
    ) {
        Ok(url) => Some(url),
        Err(err) => {
            return HttpResponse::InternalServerError().json(
                serde_json::json!({
                    "status": "error",
                    "message": format!("Failed to parse token URL: {err}")
                }),
            )
        }
    };

    let redirect_url =
        match RedirectUrl::new(_config.azure_oauth_redirect_url.to_owned()) {
            Ok(url) => url,
            Err(err) => return HttpResponse::InternalServerError().json(
                serde_json::json!({
                    "status": "error",
                    "message": format!("Failed to parse redirect URL: {err}")
                }),
            ),
        };

    let client = BasicClient::new(
        ClientId::new(_config.azure_oauth_client_id.to_owned()),
        Some(ClientSecret::new(
            _config.azure_oauth_client_secret.to_owned(),
        )),
        authorization_url,
        token_url,
    )
    // Set the URL the user will be redirected to after the authorization
    // process.
    .set_redirect_uri(redirect_url);

    let pkce_verifier = PkceCodeVerifier::new(req.code_verifier);

    match client
        .exchange_code(AuthorizationCode::new(req.code.to_string()))
        .set_pkce_verifier(pkce_verifier)
        .request_async(async_http_client)
        .await
    {
        Ok(token) => {
            return HttpResponse::Ok()
                //.append_header((
                //    LOCATION,
                //    format!("{}{}", data.env.client_origin.to_owned(), state),
                //))
                .json(serde_json::json!({
                    "status": "success",
                    "token": token
                }));
        }
        Err(err) => {
            println!("Failed to contact token endpoint: {err}");
            return HttpResponse::BadGateway().json(serde_json::json!({
                "status": "fail",
                "message": err.to_string(),
            }));
        }
    }
}

pub fn configure(conf: &mut web::ServiceConfig) {
    conf.service(token);
}
