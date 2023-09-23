use crate::modules::UserFetchingModule;

use actix_web::{get, web, HttpResponse, Responder};
use log::warn;
use myc_core::{
    domain::entities::UserFetching,
    use_cases::roles::default_users::user::check_email_registration_status,
};
use myc_http_tools::{utils::JsonError, Email};
use serde::Deserialize;
use shaku_actix::Inject;
use utoipa::IntoParams;

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

//#[derive(Deserialize, ToSchema)]
//#[serde(rename_all = "camelCase")]
//pub struct CreateDefaultUserBody {
//    email: String,
//    first_name: Option<String>,
//    last_name: Option<String>,
//    password: Option<String>,
//    provider_name: Option<String>,
//}

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

/* #[utoipa::path(
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
            description = "Account successfully created.",
            body = Account,
        ),
        (
            status = 200,
            description = "Account already exists.",
            body = Account,
        ),
    ),
)]
#[post("/")]
pub async fn create_default_user_url(
    body: web::Json<CreateDefaultUserBody>,
    user_fetching_repo: Inject<UserFetchingModule, dyn UserFetching>,
    user_registration_repo: Inject<
        UserRegistrationModule,
        dyn UserRegistration,
    >,
    user_deletion_repo: Inject<UserDeletionModule, dyn UserDeletion>,
    token_registration_repo: Inject<
        SessionTokenRegistrationModule,
        dyn SessionTokenRegistration,
) -> impl Responder {
    match create_default_user(
        body.email.to_owned(),
        body.first_name.to_owned(),
        body.last_name.to_owned(),
        body.password.to_owned(),
        body.provider_name.to_owned(),
        Box::new(&*user_fetching_repo),
        Box::new(&*account_registration_repo),
        Box::new(&*account_type_registration_repo),
    )
    .await
    {
        Err(err) => HttpResponse::InternalServerError()
            .json(JsonError::new(err.to_string())),
        Ok(res) => HttpResponse::Created().json(res),
    }
} */
