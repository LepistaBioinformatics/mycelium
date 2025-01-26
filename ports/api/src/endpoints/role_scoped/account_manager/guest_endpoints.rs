use crate::{
    dtos::{MyceliumProfileData, TenantData},
    modules::MessageSendingQueueModule,
};

use actix_web::{post, web, HttpResponse, Responder};
use myc_core::{
    domain::{dtos::guest_user::GuestUser, entities::MessageSending},
    models::AccountLifeCycle,
    use_cases::role_scoped::account_manager::guest::guest_to_children_account,
};
use myc_diesel::repositories::AppModule;
use myc_http_tools::{
    utils::HttpJsonResponse,
    wrappers::default_response_to_http_response::{
        get_or_create_response_kind, handle_mapped_error,
    },
    Email,
};
use serde::Deserialize;
use shaku::HasComponent;
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
pub struct GuestUserToChildrenBody {
    /// The email of the guest user
    email: String,

    /// The parent role id
    ///
    /// The parent related to the guest role to be created. Example, if the
    /// guest role is a child of the account manager role, the parent role id
    /// should be this role id.
    ///
    /// The child role id should be passed as the `role_id` path argument.
    parent_role_id: Uuid,
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
    params(
        (
            "x-mycelium-tenant-id" = Uuid,
            Header,
            description = "The tenant unique id."
        ),
        ("account_id" = Uuid, Path, description = "The account primary key."),
        ("role_id" = Uuid, Path, description = "The guest-role unique id."),
    ),
    request_body = GuestUserToChildrenBody,
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
#[post("/accounts/{account_id}/roles/{role_id}")]
pub async fn guest_to_children_account_url(
    tenant: TenantData,
    path: web::Path<(Uuid, Uuid)>,
    body: web::Json<GuestUserToChildrenBody>,
    profile: MyceliumProfileData,
    life_cycle_settings: web::Data<AccountLifeCycle>,
    app_module: web::Data<AppModule>,
    message_sending_repo: Inject<MessageSendingQueueModule, dyn MessageSending>,
) -> impl Responder {
    let (account_id, role_id) = path.to_owned();

    let email = match Email::from_string(body.email.to_owned()) {
        Err(err) => {
            return HttpResponse::BadRequest()
                .json(HttpJsonResponse::new_message(err.to_string()))
        }
        Ok(res) => res,
    };

    match guest_to_children_account(
        profile.to_profile(),
        tenant.tenant_id().to_owned(),
        email,
        body.parent_role_id.to_owned(),
        role_id,
        account_id,
        life_cycle_settings.get_ref().to_owned(),
        Box::new(&*app_module.resolve_ref()),
        Box::new(&*app_module.resolve_ref()),
        Box::new(&*app_module.resolve_ref()),
        Box::new(&*message_sending_repo),
    )
    .await
    {
        Ok(res) => get_or_create_response_kind(res),
        Err(err) => handle_mapped_error(err),
    }
}
