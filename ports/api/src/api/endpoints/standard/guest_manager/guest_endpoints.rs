use crate::{
    endpoints::{shared::UrlGroup, standard::shared::build_actor_context},
    modules::{
        AccountFetchingModule, GuestUserDeletionModule,
        GuestUserFetchingModule, GuestUserOnAccountUpdatingModule,
        GuestUserRegistrationModule, LicensedResourcesFetchingModule,
        MessageSendingModule,
    },
};

use actix_web::{delete, get, patch, post, web, HttpResponse, Responder};
use clean_base::entities::{
    DeletionResponseKind, FetchManyResponseKind, GetOrCreateResponseKind,
    UpdatingResponseKind,
};
use myc_core::{
    domain::{
        actors::DefaultActor,
        dtos::email::Email,
        entities::{
            AccountFetching, GuestUserDeletion, GuestUserFetching,
            GuestUserOnAccountUpdating, GuestUserRegistration,
            LicensedResourcesFetching, MessageSending,
        },
    },
    use_cases::roles::standard::guest_manager::guest::{
        guest_user, list_guest_on_subscription_account,
        list_licensed_accounts_of_email, uninvite_guest,
        update_user_guest_role,
    },
};
use myc_http_tools::{middleware::MyceliumProfileData, utils::JsonError};
use serde::Deserialize;
use shaku_actix::Inject;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

// ? -----------------------------------------------------------------------
// ? Configure application
// ? -----------------------------------------------------------------------

pub fn configure(config: &mut web::ServiceConfig) {
    config
        .service(list_licensed_accounts_of_email_url)
        .service(guest_user_url)
        .service(uninvite_guest_url)
        .service(update_user_guest_role_url)
        .service(list_guest_on_subscription_account_url);
}

// ? -----------------------------------------------------------------------
// ? Define API structs
// ? -----------------------------------------------------------------------

#[derive(Deserialize, ToSchema, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct GuestUserBody {
    pub email: String,
}

#[derive(Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct UpdateUserGuestRoleParams {
    pub new_guest_role_id: Uuid,
}

// ? -----------------------------------------------------------------------
// ? Define API paths
//
// Guest
//
// ? -----------------------------------------------------------------------

/// List subscription accounts which email was guest
#[utoipa::path(
    get,
    context_path = build_actor_context(DefaultActor::GuestManager, UrlGroup::Guests),
    params(
        GuestUserBody
    ),
    responses(
        (
            status = 500,
            description = "Unknown internal server error.",
            body = JsonError,
        ),
        (
            status = 204,
            description = "Not found.",
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
            description = "Fetching success.",
            body = [LicensedResources],
        ),
    ),
)]
#[get("/")]
pub async fn list_licensed_accounts_of_email_url(
    info: web::Query<GuestUserBody>,
    profile: MyceliumProfileData,
    licensed_resources_fetching_repo: Inject<
        LicensedResourcesFetchingModule,
        dyn LicensedResourcesFetching,
    >,
) -> impl Responder {
    let email = match Email::from_string(info.email.to_owned()) {
        Err(err) => {
            return HttpResponse::BadRequest()
                .json(JsonError::new(format!("Invalid email: {err}")))
        }
        Ok(res) => res,
    };

    match list_licensed_accounts_of_email(
        profile.to_profile(),
        email.to_owned(),
        Box::new(&*licensed_resources_fetching_repo),
    )
    .await
    {
        Err(err) => HttpResponse::InternalServerError()
            .json(JsonError::new(err.to_string())),
        Ok(res) => match res {
            FetchManyResponseKind::NotFound => {
                HttpResponse::NoContent().finish()
            }
            FetchManyResponseKind::Found(guests) => {
                HttpResponse::Ok().json(guests)
            }
            FetchManyResponseKind::FoundPaginated(guests) => {
                HttpResponse::Ok().json(guests)
            }
        },
    }
}

/// Guest a user to work on account.
///
/// This action gives the ability of the target account (specified through
/// the `account` argument) to perform actions specified in the `role`
/// path argument.
#[utoipa::path(
    post,
    context_path = build_actor_context(DefaultActor::GuestManager, UrlGroup::Guests),
    params(
        ("account" = Uuid, Path, description = "The account primary key."),
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
            body = GuestUser,
        ),
        (
            status = 200,
            description = "Guest already exist.",
            body = GuestUser,
        ),
    ),
)]
#[post("/account/{account}/role/{role}")]
pub async fn guest_user_url(
    path: web::Path<(Uuid, Uuid)>,
    body: web::Json<GuestUserBody>,
    profile: MyceliumProfileData,
    account_fetching_repo: Inject<AccountFetchingModule, dyn AccountFetching>,
    guest_registration_repo: Inject<
        GuestUserRegistrationModule,
        dyn GuestUserRegistration,
    >,
    message_sending_repo: Inject<MessageSendingModule, dyn MessageSending>,
) -> impl Responder {
    let (account_id, role_id) = path.to_owned();

    let email = match Email::from_string(body.email.to_owned()) {
        Err(err) => {
            return HttpResponse::BadRequest()
                .json(JsonError::new(err.to_string()))
        }
        Ok(res) => res,
    };

    match guest_user(
        profile.to_profile(),
        email,
        role_id,
        account_id,
        Box::new(&*account_fetching_repo),
        Box::new(&*guest_registration_repo),
        Box::new(&*message_sending_repo),
    )
    .await
    {
        Err(err) => HttpResponse::InternalServerError()
            .json(JsonError::new(err.to_string())),
        Ok(res) => match res {
            GetOrCreateResponseKind::NotCreated(guest, _) => {
                HttpResponse::Ok().json(guest)
            }
            GetOrCreateResponseKind::Created(guest) => {
                HttpResponse::Created().json(guest)
            }
        },
    }
}

/// Update guest-role of a single user.
///
/// This action gives the ability of the target account (specified through
/// the `account` argument) to replace the current specified `role` by the
/// new role.
#[utoipa::path(
    patch,
    context_path = build_actor_context(DefaultActor::GuestManager, UrlGroup::Guests),
    params(
        ("account" = Uuid, Path, description = "The account primary key."),
        ("role" = Uuid, Path, description = "The guest-role unique id."),
        UpdateUserGuestRoleParams,
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
            status = 201,
            description = "Guesting done.",
            body = GuestUser,
        ),
    ),
)]
#[patch("/account/{account}/role/{role}")]
pub async fn update_user_guest_role_url(
    path: web::Path<(Uuid, Uuid)>,
    info: web::Query<UpdateUserGuestRoleParams>,
    profile: MyceliumProfileData,
    guest_user_on_account_updating_repo: Inject<
        GuestUserOnAccountUpdatingModule,
        dyn GuestUserOnAccountUpdating,
    >,
) -> impl Responder {
    let (account_id, role_id) = path.to_owned();

    match update_user_guest_role(
        profile.to_profile(),
        role_id,
        account_id,
        info.new_guest_role_id.to_owned(),
        Box::new(&*guest_user_on_account_updating_repo),
    )
    .await
    {
        Err(err) => HttpResponse::InternalServerError()
            .json(JsonError::new(err.to_string())),
        Ok(res) => match res {
            UpdatingResponseKind::NotUpdated(_, msg) => {
                HttpResponse::Ok().json(JsonError::new(msg))
            }
            UpdatingResponseKind::Updated(guest) => {
                HttpResponse::Created().json(guest)
            }
        },
    }
}

/// Uninvite user to perform a role to account
#[utoipa::path(
    delete,
    context_path = build_actor_context(DefaultActor::GuestManager, UrlGroup::Guests),
    params(
        ("account" = Uuid, Path, description = "The account primary key."),
        ("role" = Uuid, Path, description = "The guest-role unique id."),
        GuestUserBody,
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
            description = "Guest User not uninvited.",
            body = JsonError,
        ),
        (
            status = 204,
            description = "Guest User uninvited.",
        ),
    ),
)]
#[delete("/account/{account}/role/{role}")]
pub async fn uninvite_guest_url(
    path: web::Path<(Uuid, Uuid)>,
    info: web::Query<GuestUserBody>,
    profile: MyceliumProfileData,
    guest_user_deletion_repo: Inject<
        GuestUserDeletionModule,
        dyn GuestUserDeletion,
    >,
) -> impl Responder {
    let (account_id, role_id) = path.to_owned();

    match uninvite_guest(
        profile.to_profile(),
        account_id,
        role_id,
        info.email.to_owned(),
        Box::new(&*guest_user_deletion_repo),
    )
    .await
    {
        Err(err) => HttpResponse::InternalServerError()
            .json(JsonError::new(err.to_string())),
        Ok(res) => match res {
            DeletionResponseKind::NotDeleted(_, msg) => {
                HttpResponse::Conflict().json(JsonError::new(msg))
            }
            DeletionResponseKind::Deleted => HttpResponse::NoContent().finish(),
        },
    }
}

/// List guest accounts related to a subscription account
///
/// This action fetches all non-subscription accounts related to the
/// informed subscription account.
#[utoipa::path(
    get,
    context_path = build_actor_context(DefaultActor::GuestManager, UrlGroup::Guests),
    params(
        ("account" = Uuid, Path, description = "The account primary key."),
    ),
    responses(
        (
            status = 500,
            description = "Unknown internal server error.",
            body = JsonError,
        ),
        (
            status = 204,
            description = "Not found.",
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
            description = "Fetching success.",
            body = GuestUser,
        ),
    ),
)]
#[get("/account/{account}")]
pub async fn list_guest_on_subscription_account_url(
    path: web::Path<Uuid>,
    profile: MyceliumProfileData,
    account_fetching_repo: Inject<AccountFetchingModule, dyn AccountFetching>,
    guest_user_fetching_repo: Inject<
        GuestUserFetchingModule,
        dyn GuestUserFetching,
    >,
) -> impl Responder {
    let account_id = path.to_owned();

    match list_guest_on_subscription_account(
        profile.to_profile(),
        account_id.to_owned(),
        Box::new(&*account_fetching_repo),
        Box::new(&*guest_user_fetching_repo),
    )
    .await
    {
        Err(err) => HttpResponse::InternalServerError()
            .json(JsonError::new(err.to_string())),
        Ok(res) => match res {
            FetchManyResponseKind::NotFound => {
                HttpResponse::NoContent().finish()
            }
            FetchManyResponseKind::Found(guests) => {
                HttpResponse::Ok().json(guests)
            }
            FetchManyResponseKind::FoundPaginated(guests) => {
                HttpResponse::Ok().json(guests)
            }
        },
    }
}
