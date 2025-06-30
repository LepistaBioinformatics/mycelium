use crate::dtos::{MyceliumProfileData, TenantData};

use actix_web::{delete, post, web, HttpResponse, Responder};
use myc_core::{
    domain::dtos::{email::Email, guest_user::GuestUser},
    models::AccountLifeCycle,
    use_cases::role_scoped::tenant_manager::{
        guest_user_to_subscription_manager_account,
        revoke_user_guest_to_subscription_manager_account,
    },
};
use myc_diesel::repositories::SqlAppModule;
use myc_http_tools::{
    utils::HttpJsonResponse,
    wrappers::default_response_to_http_response::{
        delete_response_kind, get_or_create_response_kind, handle_mapped_error,
    },
    Permission,
};
use serde::Deserialize;
use shaku::HasComponent;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

// ? ---------------------------------------------------------------------------
// ? Configure application
// ? ---------------------------------------------------------------------------

pub fn configure(config: &mut web::ServiceConfig) {
    config
        .service(guest_user_to_subscription_manager_account_url)
        .service(revoke_user_guest_to_subscription_manager_account_url);
}

// ? ---------------------------------------------------------------------------
// ? Define API structs
// ? ---------------------------------------------------------------------------

#[derive(Deserialize, ToSchema, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct GuestUserToSubscriptionManagerAccountBody {
    email: String,
    permission: Permission,
}

#[derive(Deserialize, ToSchema, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct RevokeUserGuestToSubscriptionManagerAccountParams {
    email: String,
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
    ),
    request_body = GuestUserToSubscriptionManagerAccountBody,
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
#[post("/accounts/{account_id}")]
pub async fn guest_user_to_subscription_manager_account_url(
    tenant: TenantData,
    path: web::Path<Uuid>,
    body: web::Json<GuestUserToSubscriptionManagerAccountBody>,
    profile: MyceliumProfileData,
    life_cycle_settings: web::Data<AccountLifeCycle>,
    sql_app_module: web::Data<SqlAppModule>,
) -> impl Responder {
    let account_id = path.to_owned();

    let email = match Email::from_string(body.email.to_owned()) {
        Err(err) => {
            return HttpResponse::BadRequest()
                .json(HttpJsonResponse::new_message(err.to_string()))
        }
        Ok(res) => res,
    };

    match guest_user_to_subscription_manager_account(
        profile.to_profile(),
        email,
        tenant.tenant_id().to_owned(),
        body.permission.to_owned(),
        account_id,
        life_cycle_settings.get_ref().to_owned(),
        Box::new(&*sql_app_module.resolve_ref()),
        Box::new(&*sql_app_module.resolve_ref()),
        Box::new(&*sql_app_module.resolve_ref()),
    )
    .await
    {
        Ok(res) => get_or_create_response_kind(res),
        Err(err) => handle_mapped_error(err),
    }
}

/// Uninvite user to perform a role to account
#[utoipa::path(
    delete,
    params(
        (
            "x-mycelium-tenant-id" = Uuid,
            Header,
            description = "The tenant unique id."
        ),
        ("account_id" = Uuid, Path, description = "The account primary key."),
        ("role_id" = Uuid, Path, description = "The guest-role unique id."),
        RevokeUserGuestToSubscriptionManagerAccountParams,
    ),
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
            description = "Guest User not uninvited.",
            body = HttpJsonResponse,
        ),
        (
            status = 204,
            description = "Guest User uninvited.",
        ),
    ),
)]
#[delete("/accounts/{account_id}/roles/{role_id}")]
pub async fn revoke_user_guest_to_subscription_manager_account_url(
    tenant: TenantData,
    path: web::Path<(Uuid, Uuid)>,
    query: web::Query<RevokeUserGuestToSubscriptionManagerAccountParams>,
    profile: MyceliumProfileData,
    app_module: web::Data<SqlAppModule>,
) -> impl Responder {
    let (account_id, role_id) = path.to_owned();

    let email = match Email::from_string(query.email.to_owned()) {
        Err(err) => {
            return HttpResponse::BadRequest()
                .json(HttpJsonResponse::new_message(err.to_string()))
        }
        Ok(res) => res,
    };

    match revoke_user_guest_to_subscription_manager_account(
        profile.to_profile(),
        tenant.tenant_id().to_owned(),
        account_id,
        role_id,
        email,
        Box::new(&*app_module.resolve_ref()),
    )
    .await
    {
        Ok(res) => delete_response_kind(res),
        Err(err) => handle_mapped_error(err),
    }
}
