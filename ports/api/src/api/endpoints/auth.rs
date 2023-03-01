use actix_session::Session;
use actix_web::{get, web, HttpResponse, Responder};
use log::warn;
use oauth2::{basic::BasicClient, AuthorizationCode, CsrfToken};
use serde::Deserialize;
use utoipa::IntoParams;

#[derive(Debug)]
pub struct AppState {
    pub oauth: BasicClient,
}

#[derive(Debug, IntoParams, Deserialize)]
pub struct AuthRequest {
    #[serde(alias = "access_token")]
    code: String,
    state: String,
    scope: Option<String>,
}

#[get("")]
async fn auth_url(
    session: Session,
    data: web::Data<AppState>,
    params: web::Query<AuthRequest>,
) -> impl Responder {
    let code = AuthorizationCode::new(params.code.clone());
    warn!("code: {:?}", code.secret());

    let state = CsrfToken::new(params.state.clone());
    let _scope = params.scope.clone();

    // Exchange the code with a token.
    let token = &data.oauth.exchange_code(code);

    warn!("token: {:?}", token);

    let token2 = &data.oauth;

    warn!("token2: {:?}", token2);

    session.insert("login", true).unwrap();

    HttpResponse::Ok().body(format!(
        r#"<html>
            <head>
                <title>
                    OAuth2 Test
                </title>
            </head>
            <body>
                Google returned the following state:
                <pre>{}</pre>
                Google returned the following token:
                <pre>{:?}</pre>
            </body>
        </html>"#,
        state.secret(),
        token
    ))
}
