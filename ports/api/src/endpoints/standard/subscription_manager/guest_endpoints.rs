use crate::{
    dtos::MyceliumProfileData,
    endpoints::{shared::UrlGroup, standard::shared::build_actor_context},
    modules::{
        AccountFetchingModule, GuestRoleFetchingModule,
        GuestUserDeletionModule, GuestUserFetchingModule,
        GuestUserOnAccountUpdatingModule, GuestUserRegistrationModule,
        LicensedResourcesFetchingModule, MessageSendingQueueModule,
    },
};

use actix_web::{delete, get, patch, post, web, HttpResponse, Responder};
use myc_core::{
    domain::{
        actors::ActorName,
        dtos::{email::Email, native_error_codes::NativeErrorCodes},
        entities::{
            AccountFetching, GuestRoleFetching, GuestUserDeletion,
            GuestUserFetching, GuestUserOnAccountUpdating,
            GuestUserRegistration, LicensedResourcesFetching, MessageSending,
        },
    },
    models::AccountLifeCycle,
    use_cases::roles::standard::subscription_manager::guest::{
        guest_user, list_guest_on_subscription_account,
        list_licensed_accounts_of_email, uninvite_guest,
        update_user_guest_role,
    },
};
use myc_http_tools::{
    utils::HttpJsonResponse,
    wrappers::default_response_to_http_response::{
        delete_response_kind, fetch_many_response_kind,
        get_or_create_response_kind, updating_response_kind,
    },
};
use serde::Deserialize;
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
        .service(update_user_guest_role_url)
        .service(list_guest_on_subscription_account_url);
}

// ? ---------------------------------------------------------------------------
// ? Define API structs
// ? ---------------------------------------------------------------------------

#[derive(Deserialize, ToSchema, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct GuestUserBody {
    email: String,
    platform_url: Option<String>,
}

#[derive(Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct UpdateUserGuestRoleParams {
    new_guest_role_id: Uuid,
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
    context_path = build_actor_context(ActorName::SubscriptionManager, UrlGroup::Guests),
    params(
        ("tenant_id" = Uuid, Path, description = "The tenant primary key."),
        GuestUserBody
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
#[get("/{tenant_id}")]
pub async fn list_licensed_accounts_of_email_url(
    path: web::Path<Uuid>,
    info: web::Query<GuestUserBody>,
    profile: MyceliumProfileData,
    licensed_resources_fetching_repo: Inject<
        LicensedResourcesFetchingModule,
        dyn LicensedResourcesFetching,
    >,
) -> impl Responder {
    let email = match Email::from_string(info.email.to_owned()) {
        Err(err) => {
            return HttpResponse::BadRequest().json(
                HttpJsonResponse::new_message(format!("Invalid email: {err}")),
            )
        }
        Ok(res) => res,
    };

    match list_licensed_accounts_of_email(
        profile.to_profile(),
        *path,
        email.to_owned(),
        Box::new(&*licensed_resources_fetching_repo),
    )
    .await
    {
        Ok(res) => fetch_many_response_kind(res),
        Err(err) => HttpResponse::InternalServerError()
            .json(HttpJsonResponse::new_message(err.to_string())),
    }
}

/// Guest a user to work on account.
///
/// This action gives the ability of the target account (specified through
/// the `account` argument) to perform actions specified in the `role`
/// path argument.
#[utoipa::path(
    post,
    context_path = build_actor_context(ActorName::SubscriptionManager, UrlGroup::Guests),
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
#[post("/{tenant_id}/accounts/{account_id}/role/{role_id}")]
pub async fn guest_user_url(
    path: web::Path<(Uuid, Uuid, Uuid)>,
    body: web::Json<GuestUserBody>,
    profile: MyceliumProfileData,
    life_cycle_settings: web::Data<AccountLifeCycle>,
    account_fetching_repo: Inject<AccountFetchingModule, dyn AccountFetching>,
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
    let (tenant_id, account_id, role_id) = path.to_owned();

    let email = match Email::from_string(body.email.to_owned()) {
        Err(err) => {
            return HttpResponse::BadRequest()
                .json(HttpJsonResponse::new_message(err.to_string()))
        }
        Ok(res) => res,
    };

    match guest_user(
        profile.to_profile(),
        tenant_id,
        email,
        role_id,
        account_id,
        body.platform_url.to_owned(),
        life_cycle_settings.get_ref().to_owned(),
        Box::new(&*account_fetching_repo),
        Box::new(&*guest_role_fetching_repo),
        Box::new(&*guest_registration_repo),
        Box::new(&*message_sending_repo),
    )
    .await
    {
        Ok(res) => get_or_create_response_kind(res),
        Err(err) => {
            let code_string = err.code().to_string();

            if err.is_in(vec![NativeErrorCodes::MYC00017]) {
                return HttpResponse::Conflict().json(
                    HttpJsonResponse::new_message(err.to_string())
                        .with_code(code_string),
                );
            }

            HttpResponse::InternalServerError()
                .json(HttpJsonResponse::new_message(err.to_string()))
        }
    }
}

/// Update guest-role of a single user.
///
/// This action gives the ability of the target account (specified through
/// the `account` argument) to replace the current specified `role` by the
/// new role.
#[utoipa::path(
    patch,
    context_path = build_actor_context(ActorName::SubscriptionManager, UrlGroup::Guests),
    params(
        ("tenant_id" = Uuid, Path, description = "The tenant primary key."),
        ("account_id" = Uuid, Path, description = "The account primary key."),
        ("role_id" = Uuid, Path, description = "The guest-role unique id."),
        UpdateUserGuestRoleParams,
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
            description = "Bad request.",
            body = HttpJsonResponse,
        ),
        (
            status = 201,
            description = "Guesting done.",
            body = GuestUser,
        ),
    ),
)]
#[patch("/{tenant_id}/accounts/{account_id}/role/{role_id}")]
pub async fn update_user_guest_role_url(
    path: web::Path<(Uuid, Uuid, Uuid)>,
    info: web::Query<UpdateUserGuestRoleParams>,
    profile: MyceliumProfileData,
    guest_user_on_account_updating_repo: Inject<
        GuestUserOnAccountUpdatingModule,
        dyn GuestUserOnAccountUpdating,
    >,
) -> impl Responder {
    let (tenant_id, account_id, role_id) = path.to_owned();

    match update_user_guest_role(
        profile.to_profile(),
        tenant_id,
        account_id,
        role_id,
        info.new_guest_role_id.to_owned(),
        Box::new(&*guest_user_on_account_updating_repo),
    )
    .await
    {
        Ok(res) => updating_response_kind(res),
        Err(err) => HttpResponse::InternalServerError()
            .json(HttpJsonResponse::new_message(err.to_string())),
    }
}

/// Uninvite user to perform a role to account
#[utoipa::path(
    delete,
    context_path = build_actor_context(ActorName::SubscriptionManager, UrlGroup::Guests),
    params(
        ("tenant_id" = Uuid, Path, description = "The tenant primary key."),
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
#[delete("/{tenant_id}/accounts/{account_id}/role/{role_id}")]
pub async fn uninvite_guest_url(
    path: web::Path<(Uuid, Uuid, Uuid)>,
    info: web::Query<GuestUserBody>,
    profile: MyceliumProfileData,
    guest_user_deletion_repo: Inject<
        GuestUserDeletionModule,
        dyn GuestUserDeletion,
    >,
) -> impl Responder {
    let (tenant_id, account_id, role_id) = path.to_owned();

    match uninvite_guest(
        profile.to_profile(),
        tenant_id,
        account_id,
        role_id,
        info.email.to_owned(),
        Box::new(&*guest_user_deletion_repo),
    )
    .await
    {
        Ok(res) => delete_response_kind(res),
        Err(err) => HttpResponse::InternalServerError()
            .json(HttpJsonResponse::new_message(err.to_string())),
    }
}

/// List guest accounts related to a subscription account
///
/// This action fetches all non-subscription accounts related to the
/// informed subscription account.
#[utoipa::path(
    get,
    context_path = build_actor_context(ActorName::SubscriptionManager, UrlGroup::Guests),
    params(
        ("tenant_id" = Uuid, Path, description = "The tenant primary key."),
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
#[get("/{tenant_id}/accounts/{account_id}")]
pub async fn list_guest_on_subscription_account_url(
    path: web::Path<(Uuid, Uuid)>,
    profile: MyceliumProfileData,
    account_fetching_repo: Inject<AccountFetchingModule, dyn AccountFetching>,
    guest_user_fetching_repo: Inject<
        GuestUserFetchingModule,
        dyn GuestUserFetching,
    >,
) -> impl Responder {
    let (tenant_id, account_id) = path.to_owned();

    match list_guest_on_subscription_account(
        profile.to_profile(),
        tenant_id,
        account_id,
        Box::new(&*account_fetching_repo),
        Box::new(&*guest_user_fetching_repo),
    )
    .await
    {
        Ok(res) => fetch_many_response_kind(res),
        Err(err) => HttpResponse::InternalServerError()
            .json(HttpJsonResponse::new_message(err.to_string())),
    }
}
