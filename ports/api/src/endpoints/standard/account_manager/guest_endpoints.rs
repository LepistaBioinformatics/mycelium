use crate::{
    dtos::MyceliumProfileData,
    endpoints::{shared::UrlGroup, standard::shared::build_actor_context},
    modules::{
        AccountFetchingModule, GuestRoleFetchingModule,
        GuestUserRegistrationModule, MessageSendingQueueModule,
    },
};

use actix_web::{post, web, HttpResponse, Responder};
use myc_core::{
    domain::{
        actors::ActorName,
        dtos::native_error_codes::NativeErrorCodes,
        entities::{
            AccountFetching, GuestRoleFetching, GuestUserRegistration,
            MessageSending,
        },
    },
    models::AccountLifeCycle,
    use_cases::roles::standard::account_manager::guest::guest_to_children_account,
};
use myc_http_tools::{
    utils::HttpJsonResponse,
    wrappers::default_response_to_http_response::{
        get_or_create_response_kind, handle_mapped_error,
    },
    Email,
};
use serde::Deserialize;
use shaku_actix::Inject;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

// ? ---------------------------------------------------------------------------
// ? Configure application
// ? ---------------------------------------------------------------------------

pub fn configure(config: &mut web::ServiceConfig) {
    config.service(guest_to_children_account_url);
}

// ? ---------------------------------------------------------------------------
// ? Define API structs
// ? ---------------------------------------------------------------------------

#[derive(Deserialize, ToSchema, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct GuestUserBody {
    email: String,
    parent_role_id: Uuid,
    platform_url: Option<String>,
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
    context_path = build_actor_context(ActorName::AccountManager, UrlGroup::Guests),
    params(
        ("tenant_id" = Uuid, Path, description = "The tenant primary key."),
        ("account_id" = Uuid, Path, description = "The account primary key."),
        ("role_id" = Uuid, Path, description = "The guest-role unique id."),
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
            body = GuestUser,
        ),
        (
            status = 200,
            description = "Guest already exist.",
            body = GuestUser,
        ),
    ),
)]
#[post("/{tenant_id}/accounts/{account_id}/roles/{role_id}")]
pub async fn guest_to_children_account_url(
    path: web::Path<(Uuid, Uuid, Uuid)>,
    body: web::Json<GuestUserBody>,
    profile: MyceliumProfileData,
    life_cycle_settings: web::Data<AccountLifeCycle>,
    account_fetching_repo: Inject<AccountFetchingModule, dyn AccountFetching>,
    guest_role_fetching_repo: Inject<
        GuestRoleFetchingModule,
        dyn GuestRoleFetching,
    >,
    guest_user_registration_repo: Inject<
        GuestUserRegistrationModule,
        dyn GuestUserRegistration,
    >,
    message_sending_repo: Inject<MessageSendingQueueModule, dyn MessageSending>,
) -> impl Responder {
    let (tenant_id, account_id, role_id) = path.to_owned();

    let email = match Email::from_string(body.email.to_owned()) {
        Err(err) => {
            return HttpResponse::BadRequest()
                .json(HttpJsonResponse::new_message(err.to_string()))
        }
        Ok(res) => res,
    };

    match guest_to_children_account(
        profile.to_profile(),
        tenant_id,
        email,
        body.parent_role_id.to_owned(),
        role_id,
        account_id,
        body.platform_url.to_owned(),
        life_cycle_settings.get_ref().to_owned(),
        Box::new(&*account_fetching_repo),
        Box::new(&*guest_role_fetching_repo),
        Box::new(&*guest_user_registration_repo),
        Box::new(&*message_sending_repo),
    )
    .await
    {
        Ok(res) => get_or_create_response_kind(res),
        Err(err) => handle_mapped_error(err),
    }
}
