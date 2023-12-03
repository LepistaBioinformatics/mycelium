use crate::modules::{
    AccountRegistrationModule, AccountTypeRegistrationModule,
    GuestUserRegistrationModule, MessageSendingModule,
};

use actix_web::{post, web, HttpResponse, Responder};
use myc_core::{
    domain::{
        dtos::{account::Account, session_token::TokenSecret},
        entities::{
            AccountRegistration, AccountTypeRegistration,
            GuestUserRegistration, MessageSending,
        },
    },
    use_cases::roles::default_users::guest::guest_to_default_account,
};
use myc_http_tools::utils::JsonError;
use serde::Deserialize;
use shaku_actix::Inject;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

// ? ---------------------------------------------------------------------------
// ? Configure application
// ? ---------------------------------------------------------------------------

pub fn configure(config: &mut web::ServiceConfig) {
    config.service(web::scope("/guests").service(guest_to_default_account_url));
}

// ? ---------------------------------------------------------------------------
// ? Define API structs
// ? ---------------------------------------------------------------------------

#[derive(Deserialize, ToSchema, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct GuestUserBody {
    pub account: Account,
}

// ? ---------------------------------------------------------------------------
// ? Define API paths
//
// Guest
//
// ? ---------------------------------------------------------------------------

/// Guest a user to work on account.
///
/// This action gives the ability of the target account (specified through
/// the `account` argument) to perform actions specified in the `role`
/// path argument.
#[utoipa::path(
    post,
    context_path = "/myc/default-users/guests",
    params(
        ("role" = Uuid, Path, description = "The guest-role unique id."),
    ),
    request_body = GuestUserBody,
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
            status = 201,
            description = "Guesting done.",
            body = Account,
        ),
        (
            status = 200,
            description = "Guest already exist.",
            body = Account,
        ),
    ),
)]
#[post("/role/{role}")]
pub async fn guest_to_default_account_url(
    path: web::Path<(Uuid,)>,
    body: web::Json<GuestUserBody>,
    token: web::Data<TokenSecret>,
    account_type_registration_repo: Inject<
        AccountTypeRegistrationModule,
        dyn AccountTypeRegistration,
    >,
    account_registration_repo: Inject<
        AccountRegistrationModule,
        dyn AccountRegistration,
    >,
    guest_registration_repo: Inject<
        GuestUserRegistrationModule,
        dyn GuestUserRegistration,
    >,
    message_sending_repo: Inject<MessageSendingModule, dyn MessageSending>,
) -> impl Responder {
    let account = body.account.to_owned();

    match guest_to_default_account(
        path.0,
        account.to_owned(),
        token.get_ref().to_owned(),
        Box::new(&*account_type_registration_repo),
        Box::new(&*account_registration_repo),
        Box::new(&*message_sending_repo),
        Box::new(&*guest_registration_repo),
    )
    .await
    {
        Err(err) => HttpResponse::InternalServerError()
            .json(JsonError::new(err.to_string())),
        Ok(_) => HttpResponse::Created().json(account),
    }
}
