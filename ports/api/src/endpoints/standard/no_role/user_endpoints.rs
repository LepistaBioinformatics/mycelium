use crate::{
    endpoints::{shared::UrlGroup, standard::shared::build_actor_context},
    middleware::parse_issuer_from_request,
    modules::{
        MessageSendingModule, TokenInvalidationModule, TokenRegistrationModule,
        UserDeletionModule, UserFetchingModule, UserRegistrationModule,
        UserUpdatingModule,
    },
};

use actix_web::{post, web, HttpRequest, HttpResponse, Responder};
use log::warn;
use myc_core::{
    domain::{
        actors::DefaultActor,
        dtos::native_error_codes::NativeErrorCodes,
        entities::{
            MessageSending, TokenInvalidation, TokenRegistration, UserDeletion,
            UserFetching, UserRegistration, UserUpdating,
        },
    },
    models::AccountLifeCycle,
    use_cases::roles::standard::no_role::user::{
        check_email_password_validity, check_email_registration_status,
        check_token_and_activate_user, create_default_user,
    },
};
use myc_http_tools::{
    functions::encode_jwt, models::internal_auth_config::InternalOauthConfig,
    responses::GatewayError, utils::JsonError, Email,
};
use serde::Deserialize;
use shaku_actix::Inject;
use std::collections::HashMap;
use utoipa::ToSchema;

// ? ---------------------------------------------------------------------------
// ? Configure application
// ? ---------------------------------------------------------------------------

pub fn configure(config: &mut web::ServiceConfig) {
    config
        .service(check_email_registration_status_url)
        .service(create_default_user_url)
        .service(check_user_token_url)
        //.service(check_password_change_token_url)
        .service(check_email_password_validity_url);
}

// ? ---------------------------------------------------------------------------
// ? Define API structs
// ? ---------------------------------------------------------------------------

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CheckEmailStatusBody {
    email: String,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateDefaultUserBody {
    email: String,
    first_name: Option<String>,
    last_name: Option<String>,
    password: Option<String>,
    platform_url: Option<String>,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CheckTokenBody {
    token: String,
    email: String,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CheckUserCredentialsBody {
    email: String,
    password: String,
}

// ? ---------------------------------------------------------------------------
// ? Define API paths
//
// User
//
// ? ---------------------------------------------------------------------------

/// Check email registration status
///
#[utoipa::path(
    post,
    context_path = build_actor_context(DefaultActor::NoRole, UrlGroup::Users),
    request_body = CheckEmailStatusBody,
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
#[post("/status/")]
pub async fn check_email_registration_status_url(
    info: web::Json<CheckEmailStatusBody>,
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
    context_path = build_actor_context(DefaultActor::NoRole, UrlGroup::Users),
    request_body = CreateDefaultUserBody,
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
    req: HttpRequest,
    body: web::Json<CreateDefaultUserBody>,
    token: web::Data<AccountLifeCycle>,
    user_registration_repo: Inject<
        UserRegistrationModule,
        dyn UserRegistration,
    >,
    user_deletion_repo: Inject<UserDeletionModule, dyn UserDeletion>,
    token_registration_repo: Inject<
        TokenRegistrationModule,
        dyn TokenRegistration,
    >,
    message_sending_repo: Inject<MessageSendingModule, dyn MessageSending>,
) -> impl Responder {
    let provider = match parse_issuer_from_request(req.clone()).await {
        Err(err) => match err {
            GatewayError::Forbidden(_) => None,
            _ => {
                warn!("Invalid issuer: {err}");

                return HttpResponse::BadRequest()
                    .json(JsonError::new("Invalid issuer.".to_string()));
            }
        },
        Ok(res) => Some(res),
    };

    match create_default_user(
        body.email.to_owned(),
        body.first_name.to_owned(),
        body.last_name.to_owned(),
        body.password.to_owned(),
        provider,
        token.get_ref().to_owned(),
        body.platform_url.to_owned(),
        Box::new(&*user_registration_repo),
        Box::new(&*token_registration_repo),
        Box::new(&*message_sending_repo),
        Box::new(&*user_deletion_repo),
    )
    .await
    {
        Err(err) => {
            let code_string = err.code().to_string();

            if err.is_in(vec![NativeErrorCodes::MYC00002]) {
                return HttpResponse::Conflict().json(
                    JsonError::new(err.to_string()).with_code(code_string),
                );
            }

            HttpResponse::InternalServerError()
                .json(JsonError::new(err.to_string()).with_code(code_string))
        }
        Ok(res) => HttpResponse::Created().json(res),
    }
}

#[utoipa::path(
    post,
    context_path = build_actor_context(DefaultActor::NoRole, UrlGroup::Users),
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
#[post("/validate-activation-token/")]
pub async fn check_user_token_url(
    body: web::Json<CheckTokenBody>,
    user_fetching_repo: Inject<UserFetchingModule, dyn UserFetching>,
    user_updating_repo: Inject<UserUpdatingModule, dyn UserUpdating>,
    token_invalidation_repo: Inject<
        TokenInvalidationModule,
        dyn TokenInvalidation,
    >,
) -> impl Responder {
    let email = match Email::from_string(body.email.to_owned()) {
        Err(err) => {
            warn!("Invalid email: {}", err);
            return HttpResponse::BadRequest()
                .json(JsonError::new("Invalid email address.".to_string()));
        }
        Ok(email) => email,
    };

    match check_token_and_activate_user(
        body.token.to_owned(),
        email,
        Box::new(&*user_fetching_repo),
        Box::new(&*user_updating_repo),
        Box::new(&*token_invalidation_repo),
    )
    .await
    {
        Err(err) => {
            let code_string = err.code().to_string();

            if err.is_in(vec![NativeErrorCodes::MYC00002]) {
                return HttpResponse::Conflict().json(
                    JsonError::new(err.to_string()).with_code(code_string),
                );
            }

            if err.is_in(vec![
                NativeErrorCodes::MYC00008,
                NativeErrorCodes::MYC00009,
            ]) {
                return HttpResponse::BadRequest().json(
                    JsonError::new(err.to_string()).with_code(code_string),
                );
            }

            HttpResponse::InternalServerError()
                .json(JsonError::new(err.to_string()).with_code(code_string))
        }
        Ok(_) => HttpResponse::Ok().finish(),
    }
}

//#[utoipa::path(
//    post,
//    context_path = build_actor_context(DefaultActor::NoRole, UrlGroup::Users),
//    request_body = CheckTokenBody,
//    responses(
//        (
//            status = 500,
//            description = "Unknown internal server error.",
//            body = JsonError,
//        ),
//        (
//            status = 403,
//            description = "Forbidden.",
//            body = JsonError,
//        ),
//        (
//            status = 401,
//            description = "Unauthorized.",
//            body = JsonError,
//        ),
//        (
//            status = 200,
//            description = "Password change token is valid.",
//            body = bool,
//        ),
//    ),
//)]
//#[post("/check-password-change-token/")]
//pub async fn check_password_change_token_url(
//    body: web::Json<CheckTokenBody>,
//    token: web::Data<TokenSecret>,
//    token_fetching_repo: Inject<TokenFetchingModule, dyn TokenFetching>,
//    token_deletion_repo: Inject<TokenDeletionModule, dyn TokenDeletion>,
//) -> impl Responder {
//    match verify_confirmation_token_pasetor(
//        body.token.to_owned(),
//        Some(true),
//        token.get_ref().to_owned(),
//        Box::new(&*token_fetching_repo),
//        Box::new(&*token_deletion_repo),
//    )
//    .await
//    {
//        Err(err) => HttpResponse::InternalServerError()
//            .json(JsonError::new(err.to_string())),
//        Ok(_) => HttpResponse::Created().json(true),
//    }
//}

#[utoipa::path(
    post,
    context_path = build_actor_context(DefaultActor::NoRole, UrlGroup::Users),
    request_body = CheckUserCredentialsBody,
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
            description = "Credentials are valid.",
            body = User,
        ),
    ),
)]
#[post("/login/")]
pub async fn check_email_password_validity_url(
    body: web::Json<CheckUserCredentialsBody>,
    user_fetching_repo: Inject<UserFetchingModule, dyn UserFetching>,
    token: web::Data<InternalOauthConfig>,
) -> impl Responder {
    let email_instance = match Email::from_string(body.email.to_owned()) {
        Err(err) => {
            warn!("Invalid email: {}", err);
            return HttpResponse::BadRequest()
                .json(JsonError::new("Invalid email address.".to_string()));
        }
        Ok(email) => email,
    };

    match check_email_password_validity(
        email_instance,
        body.password.to_owned(),
        Box::new(&*user_fetching_repo),
    )
    .await
    {
        Err(err) => HttpResponse::InternalServerError()
            .json(JsonError::new(err.to_string())),
        Ok((valid, user)) => match valid {
            true => {
                let _user = if let None = user {
                    return HttpResponse::NoContent().finish();
                } else {
                    user.unwrap()
                };

                let token = match encode_jwt(
                    _user.to_owned(),
                    token.get_ref().to_owned(),
                ) {
                    Err(err) => return err,
                    Ok(token) => token,
                };

                let serialized_user = match serde_json::to_string(&_user) {
                    Ok(user) => user,
                    Err(err) => {
                        return HttpResponse::InternalServerError()
                            .json(JsonError::new(err.to_string()));
                    }
                };

                HttpResponse::Ok().json(HashMap::from([
                    ("token".to_string(), token),
                    ("user".to_string(), serialized_user),
                ]))
            }
            false => HttpResponse::NoContent().finish(),
        },
    }
}
