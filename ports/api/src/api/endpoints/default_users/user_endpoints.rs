use crate::modules::{
    MessageSendingModule, SessionTokenDeletionModule,
    SessionTokenFetchingModule, SessionTokenRegistrationModule,
    UserDeletionModule, UserFetchingModule, UserRegistrationModule,
};

use actix_web::{get, http::header, post, web, HttpResponse, Responder};
use awc::error::HeaderValue;
use log::warn;
use myc_core::{
    domain::{
        dtos::session_token::TokenSecret,
        entities::{
            MessageSending, SessionTokenDeletion, SessionTokenFetching,
            SessionTokenRegistration, UserDeletion, UserFetching,
            UserRegistration,
        },
    },
    use_cases::roles::default_users::user::{
        check_email_registration_status, create_default_user,
        verify_confirmation_token_pasetor,
    },
};
use myc_http_tools::{utils::JsonError, Email};
use serde::Deserialize;
use shaku_actix::Inject;
use utoipa::{IntoParams, ToSchema};

// ? ---------------------------------------------------------------------------
// ? Configure application
// ? ---------------------------------------------------------------------------

pub fn configure(config: &mut web::ServiceConfig) {
    config.service(
        web::scope("/users").service(check_email_registration_status_url),
    );
}

// ? ---------------------------------------------------------------------------
// ? Define API structs
// ? ---------------------------------------------------------------------------

#[derive(Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct CheckEmailStatusParams {
    email: String,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateDefaultUserBody {
    email: String,
    first_name: Option<String>,
    last_name: Option<String>,
    password: Option<String>,
    provider_name: Option<String>,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CheckTokenBody {
    token: String,
    redirect_url: String,
}

// ? ---------------------------------------------------------------------------
// ? Define API paths
//
// User
//
// ? ---------------------------------------------------------------------------

#[utoipa::path(
    get,
    context_path = "/myc/default-users/users",
    params(
        CheckEmailStatusParams,
    ),
    responses(
        (
            status = 500,
            description = "Unknown internal server error.",
            body = JsonError,
        ),
        (
            status = 403,
            description = "Forbidden.",
            body = JsonError,
        ),
        (
            status = 401,
            description = "Unauthorized.",
            body = JsonError,
        ),
        (
            status = 400,
            description = "Bad request.",
            body = JsonError,
        ),
        (
            status = 204,
            description = "Not found.",
        ),
        (
            status = 200,
            description = "Status fetching done.",
            body = EmailRegistrationStatus,
        ),
    ),
)]
#[get("/")]
pub async fn check_email_registration_status_url(
    info: web::Query<CheckEmailStatusParams>,
    user_fetching_repo: Inject<UserFetchingModule, dyn UserFetching>,
) -> impl Responder {
    let email_instance = match Email::from_string(info.email.to_owned()) {
        Err(err) => {
            warn!("Invalid email: {}", err);
            return HttpResponse::BadRequest()
                .json(JsonError::new("Invalid email address.".to_string()));
        }
        Ok(email) => email,
    };

    match check_email_registration_status(
        email_instance,
        Box::new(&*user_fetching_repo),
    )
    .await
    {
        Err(err) => HttpResponse::InternalServerError()
            .json(JsonError::new(err.to_string())),
        Ok(res) => HttpResponse::Ok().json(res),
    }
}

#[utoipa::path(
    post,
    context_path = "/myc/default-users/users",
    request_body = CreateDefaultAccountBody,
    responses(
        (
            status = 500,
            description = "Unknown internal server error.",
            body = JsonError,
        ),
        (
            status = 403,
            description = "Forbidden.",
            body = JsonError,
        ),
        (
            status = 401,
            description = "Unauthorized.",
            body = JsonError,
        ),
        (
            status = 201,
            description = "User successfully created.",
            body = User,
        ),
    ),
)]
#[post("/")]
pub async fn create_default_user_url(
    body: web::Json<CreateDefaultUserBody>,
    token: web::Data<TokenSecret>,
    user_registration_repo: Inject<
        UserRegistrationModule,
        dyn UserRegistration,
    >,
    user_deletion_repo: Inject<UserDeletionModule, dyn UserDeletion>,
    token_registration_repo: Inject<
        SessionTokenRegistrationModule,
        dyn SessionTokenRegistration,
    >,
    message_sending_repo: Inject<MessageSendingModule, dyn MessageSending>,
) -> impl Responder {
    match create_default_user(
        body.email.to_owned(),
        body.first_name.to_owned(),
        body.last_name.to_owned(),
        body.password.to_owned(),
        body.provider_name.to_owned(),
        token.get_ref().to_owned(),
        Box::new(&*user_registration_repo),
        Box::new(&*user_deletion_repo),
        Box::new(&*token_registration_repo),
        Box::new(&*message_sending_repo),
    )
    .await
    {
        Err(err) => HttpResponse::InternalServerError()
            .json(JsonError::new(err.to_string())),
        Ok(res) => HttpResponse::Created().json(res),
    }
}

#[utoipa::path(
    post,
    context_path = "/myc/default-users/users",
    request_body = CheckTokenBody,
    responses(
        (
            status = 500,
            description = "Unknown internal server error.",
            body = JsonError,
        ),
        (
            status = 403,
            description = "Forbidden.",
            body = JsonError,
        ),
        (
            status = 401,
            description = "Unauthorized.",
            body = JsonError,
        ),
        (
            status = 200,
            description = "Activation token is valid.",
            body = bool,
        ),
    ),
)]
#[post("/check-account-activation-token")]
pub async fn check_activation_token_url(
    body: web::Json<CheckTokenBody>,
    token: web::Data<TokenSecret>,
    token_fetching_repo: Inject<
        SessionTokenFetchingModule,
        dyn SessionTokenFetching,
    >,
    token_deletion_repo: Inject<
        SessionTokenDeletionModule,
        dyn SessionTokenDeletion,
    >,
) -> impl Responder {
    let redirect_url = match HeaderValue::from_str(&body.redirect_url) {
        Err(err) => {
            warn!("Invalid redirect url: {}", err);
            return HttpResponse::BadRequest()
                .json(JsonError::new("Invalid redirect url.".to_string()));
        }
        Ok(res) => res,
    };

    match verify_confirmation_token_pasetor(
        body.token.to_owned(),
        None,
        token.get_ref().to_owned(),
        Box::new(&*token_fetching_repo),
        Box::new(&*token_deletion_repo),
    )
    .await
    {
        Err(err) => HttpResponse::InternalServerError()
            .json(JsonError::new(err.to_string())),
        Ok(_) => {
            let mut res = HttpResponse::TemporaryRedirect();
            res.append_header((header::LOCATION, redirect_url));
            res.finish()
        }
    }
}

#[utoipa::path(
    post,
    context_path = "/myc/default-users/users",
    request_body = CheckTokenBody,
    responses(
        (
            status = 500,
            description = "Unknown internal server error.",
            body = JsonError,
        ),
        (
            status = 403,
            description = "Forbidden.",
            body = JsonError,
        ),
        (
            status = 401,
            description = "Unauthorized.",
            body = JsonError,
        ),
        (
            status = 200,
            description = "Password change token is valid.",
            body = bool,
        ),
    ),
)]
#[post("/check-password-change-token")]
pub async fn check_password_change_token_url(
    body: web::Json<CheckTokenBody>,
    token: web::Data<TokenSecret>,
    token_fetching_repo: Inject<
        SessionTokenFetchingModule,
        dyn SessionTokenFetching,
    >,
    token_deletion_repo: Inject<
        SessionTokenDeletionModule,
        dyn SessionTokenDeletion,
    >,
) -> impl Responder {
    match verify_confirmation_token_pasetor(
        body.token.to_owned(),
        Some(true),
        token.get_ref().to_owned(),
        Box::new(&*token_fetching_repo),
        Box::new(&*token_deletion_repo),
    )
    .await
    {
        Err(err) => HttpResponse::InternalServerError()
            .json(JsonError::new(err.to_string())),
        Ok(_) => HttpResponse::Created().json(true),
    }
}
