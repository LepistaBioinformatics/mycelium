use crate::{
    endpoints::{shared::UrlGroup, standard::shared::build_actor_context},
    modules::{
        AccountRegistrationModule, GuestUserRegistrationModule,
        MessageSendingModule,
    },
};

use actix_web::{post, web, HttpResponse, Responder};
use myc_core::{
    domain::{
        actors::ActorName,
        dtos::account::Account,
        entities::{
            AccountRegistration, GuestUserRegistration, MessageSending,
        },
    },
    models::AccountLifeCycle,
    use_cases::roles::standard::no_role::guest::guest_to_default_account,
};
use myc_http_tools::utils::HttpJsonResponse;
use serde::Deserialize;
use shaku_actix::Inject;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

// ? ---------------------------------------------------------------------------
// ? Configure application
// ? ---------------------------------------------------------------------------

pub fn configure(config: &mut web::ServiceConfig) {
    config.service(guest_to_default_account_url);
}

// ? ---------------------------------------------------------------------------
// ? Define API structs
// ? ---------------------------------------------------------------------------

#[derive(Deserialize, ToSchema, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct GuestUserBody {
    account: Account,
    tenant_id: Uuid,
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
    context_path = build_actor_context(ActorName::NoRole, UrlGroup::Guests),
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
    token: web::Data<AccountLifeCycle>,
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
        body.tenant_id.to_owned(),
        token.get_ref().to_owned(),
        Box::new(&*account_registration_repo),
        Box::new(&*message_sending_repo),
        Box::new(&*guest_registration_repo),
    )
    .await
    {
        Ok(_) => HttpResponse::Created().json(account),
        Err(err) => HttpResponse::InternalServerError()
            .json(HttpJsonResponse::new_message(err.to_string())),
    }
}
