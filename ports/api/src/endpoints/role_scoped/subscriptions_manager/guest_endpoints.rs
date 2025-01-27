use crate::{
    dtos::{MyceliumProfileData, TenantData},
    modules::MessageSendingQueueModule,
};

use actix_web::{delete, get, post, web, HttpResponse, Responder};
use myc_core::{
    domain::{
        dtos::{
            email::Email, guest_user::GuestUser, profile::LicensedResources,
        },
        entities::MessageSending,
    },
    models::AccountLifeCycle,
    use_cases::role_scoped::subscriptions_manager::guest::{
        guest_user_to_subscription_account, list_guest_on_subscription_account,
        list_licensed_accounts_of_email,
        revoke_user_guest_to_subscription_account,
    },
};
use myc_diesel::repositories::SqlAppModule;
use myc_http_tools::{
    utils::HttpJsonResponse,
    wrappers::default_response_to_http_response::{
        delete_response_kind, fetch_many_response_kind,
        get_or_create_response_kind, handle_mapped_error,
    },
    Permission,
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
    config
        .service(list_licensed_accounts_of_email_url)
        .service(guest_user_url)
        .service(uninvite_guest_url)
        .service(list_guest_on_subscription_account_url);
}

// ? ---------------------------------------------------------------------------
// ? Define API structs
// ? ---------------------------------------------------------------------------

#[derive(Deserialize, ToSchema, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct GuestUserBody {
    email: String,
}

#[derive(Deserialize, ToSchema, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct ListLicensedAccountsOfEmailParams {
    /// The email which the guest user is connected to
    email: String,

    /// The roles which the guest user was invited to
    roles: Option<Vec<String>>,

    /// The permissioned roles which the guest user was invited to
    permissioned_roles: Option<Vec<(String, Permission)>>,

    /// The guest user was verified
    was_verified: Option<bool>,
}

// ? ---------------------------------------------------------------------------
// ? Define API paths
//
// Guest
//
// ? ---------------------------------------------------------------------------

/// List subscription accounts which email was guest
#[utoipa::path(
    get,
    params(
        (
            "x-mycelium-tenant-id" = Uuid,
            Header,
            description = "The tenant unique id."
        ),
        ListLicensedAccountsOfEmailParams,
    ),
    responses(
        (
            status = 500,
            description = "Unknown internal server error.",
            body = HttpJsonResponse,
        ),
        (
            status = 204,
            description = "Not found.",
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
            status = 200,
            description = "Fetching success.",
            body = [LicensedResources],
        ),
    ),
)]
#[get("")]
pub async fn list_licensed_accounts_of_email_url(
    tenant: TenantData,
    query: web::Query<ListLicensedAccountsOfEmailParams>,
    profile: MyceliumProfileData,
    app_module: web::Data<SqlAppModule>,
) -> impl Responder {
    let email = match Email::from_string(query.email.to_owned()) {
        Err(err) => {
            return HttpResponse::BadRequest().json(
                HttpJsonResponse::new_message(format!("Invalid email: {err}")),
            )
        }
        Ok(res) => res,
    };

    match list_licensed_accounts_of_email(
        profile.to_profile(),
        tenant.tenant_id().to_owned(),
        email.to_owned(),
        query.roles.to_owned(),
        query.was_verified.to_owned(),
        query.permissioned_roles.to_owned(),
        Box::new(&*app_module.resolve_ref()),
    )
    .await
    {
        Ok(res) => fetch_many_response_kind(res),
        Err(err) => handle_mapped_error(err),
    }
}

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
#[post("/accounts/{account_id}/roles/{role_id}")]
pub async fn guest_user_url(
    tenant: TenantData,
    path: web::Path<(Uuid, Uuid)>,
    body: web::Json<GuestUserBody>,
    profile: MyceliumProfileData,
    life_cycle_settings: web::Data<AccountLifeCycle>,
    app_module: web::Data<SqlAppModule>,
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

    match guest_user_to_subscription_account(
        profile.to_profile(),
        tenant.tenant_id().to_owned(),
        email,
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
        GuestUserBody,
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
pub async fn uninvite_guest_url(
    tenant: TenantData,
    path: web::Path<(Uuid, Uuid)>,
    query: web::Query<GuestUserBody>,
    profile: MyceliumProfileData,
    app_module: web::Data<SqlAppModule>,
) -> impl Responder {
    let (account_id, role_id) = path.to_owned();

    match revoke_user_guest_to_subscription_account(
        profile.to_profile(),
        tenant.tenant_id().to_owned(),
        account_id,
        role_id,
        query.email.to_owned(),
        Box::new(&*app_module.resolve_ref()),
    )
    .await
    {
        Ok(res) => delete_response_kind(res),
        Err(err) => handle_mapped_error(err),
    }
}

/// List guest accounts related to a subscription account
///
/// This action fetches all non-subscription accounts related to the
/// informed subscription account.
#[utoipa::path(
    get,
    params(
        (
            "x-mycelium-tenant-id" = Uuid,
            Header,
            description = "The tenant unique id."
        ),
        ("account_id" = Uuid, Path, description = "The account primary key."),
    ),
    responses(
        (
            status = 500,
            description = "Unknown internal server error.",
            body = HttpJsonResponse,
        ),
        (
            status = 204,
            description = "Not found.",
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
            status = 200,
            description = "Fetching success.",
            body = GuestUser,
        ),
    ),
)]
#[get("/accounts/{account_id}")]
pub async fn list_guest_on_subscription_account_url(
    tenant: TenantData,
    path: web::Path<Uuid>,
    profile: MyceliumProfileData,
    app_module: web::Data<SqlAppModule>,
) -> impl Responder {
    match list_guest_on_subscription_account(
        profile.to_profile(),
        tenant.tenant_id().to_owned(),
        path.to_owned(),
        Box::new(&*app_module.resolve_ref()),
        Box::new(&*app_module.resolve_ref()),
    )
    .await
    {
        Ok(res) => fetch_many_response_kind(res),
        Err(err) => handle_mapped_error(err),
    }
}
