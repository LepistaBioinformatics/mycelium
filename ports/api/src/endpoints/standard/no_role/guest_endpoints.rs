use crate::{
    endpoints::{shared::UrlGroup, standard::shared::build_actor_context},
    modules::{
        AccountRegistrationModule, GuestRoleFetchingModule,
        GuestUserRegistrationModule, MessageSendingQueueModule,
    },
};

use actix_web::{post, web, HttpResponse, Responder};
use myc_core::{
    domain::{
        actors::ActorName,
        dtos::account::Account,
        entities::{
            AccountRegistration, GuestRoleFetching, GuestUserRegistration,
            MessageSending,
        },
    },
    models::AccountLifeCycle,
    use_cases::roles::standard::no_role::guest::guest_to_default_account,
};
use myc_http_tools::wrappers::default_response_to_http_response::handle_mapped_error;
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
            body = HttpJsonResponse,
        ),
        (
            status = 403,
            description = "Forbidden.",
            body = HttpJsonResponse,
        ),
        (
            status = 401,
            description = "Unauthorized.",
            body = HttpJsonResponse,
        ),
        (
            status = 400,
            description = "Bad request.",
            body = HttpJsonResponse,
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
#[post("/{tenant_id}/role/{role}")]
pub async fn guest_to_default_account_url(
    path: web::Path<(Uuid, Uuid)>,
    body: web::Json<GuestUserBody>,
    life_cycle_settings: web::Data<AccountLifeCycle>,
    account_registration_repo: Inject<
        AccountRegistrationModule,
        dyn AccountRegistration,
    >,
    guest_role_fetching_repo: Inject<
        GuestRoleFetchingModule,
        dyn GuestRoleFetching,
    >,
    guest_registration_repo: Inject<
        GuestUserRegistrationModule,
        dyn GuestUserRegistration,
    >,
    message_sending_repo: Inject<MessageSendingQueueModule, dyn MessageSending>,
) -> impl Responder {
    let (tenant_id, role_id) = path.to_owned();
    let account = body.account.to_owned();

    match guest_to_default_account(
        role_id,
        account.to_owned(),
        tenant_id,
        life_cycle_settings.get_ref().to_owned(),
        Box::new(&*account_registration_repo),
        Box::new(&*guest_role_fetching_repo),
        Box::new(&*message_sending_repo),
        Box::new(&*guest_registration_repo),
    )
    .await
    {
        Ok(_) => HttpResponse::Created().json(account),
        Err(err) => handle_mapped_error(err),
    }
}
