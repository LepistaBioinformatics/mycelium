use crate::{
    dtos::MyceliumProfileData,
    endpoints::{shared::UrlGroup, standard::shared::build_actor_context},
    modules::{AccountFetchingModule, AccountUpdatingModule},
};

use actix_web::{patch, web, HttpResponse, Responder};
use myc_core::{
    domain::{
        actors::DefaultActor,
        entities::{AccountFetching, AccountUpdating},
    },
    use_cases::roles::standard::user_account_manager::account::{
        change_account_activation_status, change_account_approval_status,
        change_account_archival_status,
    },
};
use myc_http_tools::utils::JsonError;
use mycelium_base::entities::UpdatingResponseKind;
use shaku_actix::Inject;
use uuid::Uuid;

// ? ---------------------------------------------------------------------------
// ? Configure application
// ? ---------------------------------------------------------------------------

pub fn configure(config: &mut web::ServiceConfig) {
    config
        .service(approve_account_url)
        .service(disapprove_account_url)
        .service(activate_account_url)
        .service(deactivate_account_url)
        .service(archive_account_url)
        .service(unarchive_account_url);
}

// ? ---------------------------------------------------------------------------
// ? Define API paths
//
// Account
//
// ? ---------------------------------------------------------------------------

/// Approve account after creation
///
/// New accounts should be approved after has permissions to perform
/// operation on the system. These endpoint should approve such account.
#[utoipa::path(
    patch,
    context_path = build_actor_context(DefaultActor::UserAccountManager, UrlGroup::Accounts),
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
            description = "Account not approved.",
            body = JsonError,
        ),
        (
            status = 202,
            description = "Account approved.",
            body = Account,
        ),
    ),
)]
#[patch("/{account}/approve")]
pub async fn approve_account_url(
    path: web::Path<Uuid>,
    profile: MyceliumProfileData,
    account_fetching_repo: Inject<AccountFetchingModule, dyn AccountFetching>,
    account_updating_repo: Inject<AccountUpdatingModule, dyn AccountUpdating>,
) -> impl Responder {
    match change_account_approval_status(
        profile.to_profile(),
        path.to_owned(),
        true,
        Box::new(&*account_fetching_repo),
        Box::new(&*account_updating_repo),
    )
    .await
    {
        Err(err) => HttpResponse::InternalServerError()
            .json(JsonError::new(err.to_string())),
        Ok(res) => match res {
            UpdatingResponseKind::NotUpdated(_, msg) => {
                HttpResponse::BadRequest().json(JsonError::new(msg))
            }
            UpdatingResponseKind::Updated(record) => {
                HttpResponse::Accepted().json(record)
            }
        },
    }
}

/// Disapprove account after creation
///
/// Also approved account should be disapproved at any time. These endpoint
/// work for this.
#[utoipa::path(
    patch,
    context_path = build_actor_context(DefaultActor::UserAccountManager, UrlGroup::Accounts),
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
            description = "Account not disapproved.",
            body = JsonError,
        ),
        (
            status = 202,
            description = "Account disapproved.",
            body = Account,
        ),
    ),
)]
#[patch("/{account}/disapprove")]
pub async fn disapprove_account_url(
    path: web::Path<Uuid>,
    profile: MyceliumProfileData,
    account_fetching_repo: Inject<AccountFetchingModule, dyn AccountFetching>,
    account_updating_repo: Inject<AccountUpdatingModule, dyn AccountUpdating>,
) -> impl Responder {
    match change_account_approval_status(
        profile.to_profile(),
        path.to_owned(),
        false,
        Box::new(&*account_fetching_repo),
        Box::new(&*account_updating_repo),
    )
    .await
    {
        Err(err) => HttpResponse::InternalServerError()
            .json(JsonError::new(err.to_string())),
        Ok(res) => match res {
            UpdatingResponseKind::NotUpdated(_, msg) => {
                HttpResponse::BadRequest().json(JsonError::new(msg))
            }
            UpdatingResponseKind::Updated(record) => {
                HttpResponse::Accepted().json(record)
            }
        },
    }
}

/// Activate account
///
/// Any account could be activated and deactivated. This action turn an
/// account active.
#[utoipa::path(
    patch,
    context_path = build_actor_context(DefaultActor::UserAccountManager, UrlGroup::Accounts),
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
            description = "Account not activated.",
            body = JsonError,
        ),
        (
            status = 202,
            description = "Account activated.",
            body = Account,
        ),
    ),
)]
#[patch("/{account}/activate")]
pub async fn activate_account_url(
    path: web::Path<Uuid>,
    profile: MyceliumProfileData,
    account_fetching_repo: Inject<AccountFetchingModule, dyn AccountFetching>,
    account_updating_repo: Inject<AccountUpdatingModule, dyn AccountUpdating>,
) -> impl Responder {
    match change_account_activation_status(
        profile.to_profile(),
        path.to_owned(),
        true,
        Box::new(&*account_fetching_repo),
        Box::new(&*account_updating_repo),
    )
    .await
    {
        Err(err) => HttpResponse::InternalServerError()
            .json(JsonError::new(err.to_string())),
        Ok(res) => match res {
            UpdatingResponseKind::NotUpdated(_, msg) => {
                HttpResponse::BadRequest().json(JsonError::new(msg))
            }
            UpdatingResponseKind::Updated(record) => {
                HttpResponse::Accepted().json(record)
            }
        },
    }
}

/// Deactivate account
///
/// Any account could be activated and deactivated. This action turn an
/// account deactivated.
#[utoipa::path(
    patch,
    context_path = build_actor_context(DefaultActor::UserAccountManager, UrlGroup::Accounts),
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
            description = "Account not activated.",
            body = JsonError,
        ),
        (
            status = 202,
            description = "Account activated.",
            body = Account,
        ),
    ),
)]
#[patch("/{account}/deactivate")]
pub async fn deactivate_account_url(
    path: web::Path<Uuid>,
    profile: MyceliumProfileData,
    account_fetching_repo: Inject<AccountFetchingModule, dyn AccountFetching>,
    account_updating_repo: Inject<AccountUpdatingModule, dyn AccountUpdating>,
) -> impl Responder {
    match change_account_activation_status(
        profile.to_profile(),
        path.to_owned(),
        false,
        Box::new(&*account_fetching_repo),
        Box::new(&*account_updating_repo),
    )
    .await
    {
        Err(err) => HttpResponse::InternalServerError()
            .json(JsonError::new(err.to_string())),
        Ok(res) => match res {
            UpdatingResponseKind::NotUpdated(_, msg) => {
                HttpResponse::BadRequest().json(JsonError::new(msg))
            }
            UpdatingResponseKind::Updated(record) => {
                HttpResponse::Accepted().json(record)
            }
        },
    }
}

/// Archive account
///
/// Set target account as archived.
#[utoipa::path(
    patch,
    context_path = build_actor_context(DefaultActor::UserAccountManager, UrlGroup::Accounts),
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
            description = "Account not activated.",
            body = JsonError,
        ),
        (
            status = 202,
            description = "Account activated.",
            body = Account,
        ),
    ),
)]
#[patch("/{account}/archive")]
pub async fn archive_account_url(
    path: web::Path<Uuid>,
    profile: MyceliumProfileData,
    account_fetching_repo: Inject<AccountFetchingModule, dyn AccountFetching>,
    account_updating_repo: Inject<AccountUpdatingModule, dyn AccountUpdating>,
) -> impl Responder {
    match change_account_archival_status(
        profile.to_profile(),
        path.to_owned(),
        true,
        Box::new(&*account_fetching_repo),
        Box::new(&*account_updating_repo),
    )
    .await
    {
        Err(err) => HttpResponse::InternalServerError()
            .json(JsonError::new(err.to_string())),
        Ok(res) => match res {
            UpdatingResponseKind::NotUpdated(_, msg) => {
                HttpResponse::BadRequest().json(JsonError::new(msg))
            }
            UpdatingResponseKind::Updated(record) => {
                HttpResponse::Accepted().json(record)
            }
        },
    }
}

/// Unarchive account
///
/// Set target account as un-archived.
#[utoipa::path(
    patch,
    context_path = build_actor_context(DefaultActor::UserAccountManager, UrlGroup::Accounts),
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
            description = "Account not activated.",
            body = JsonError,
        ),
        (
            status = 202,
            description = "Account activated.",
            body = Account,
        ),
    ),
)]
#[patch("/{account}/unarchive")]
pub async fn unarchive_account_url(
    path: web::Path<Uuid>,
    profile: MyceliumProfileData,
    account_fetching_repo: Inject<AccountFetchingModule, dyn AccountFetching>,
    account_updating_repo: Inject<AccountUpdatingModule, dyn AccountUpdating>,
) -> impl Responder {
    match change_account_archival_status(
        profile.to_profile(),
        path.to_owned(),
        false,
        Box::new(&*account_fetching_repo),
        Box::new(&*account_updating_repo),
    )
    .await
    {
        Err(err) => HttpResponse::InternalServerError()
            .json(JsonError::new(err.to_string())),
        Ok(res) => match res {
            UpdatingResponseKind::NotUpdated(_, msg) => {
                HttpResponse::BadRequest().json(JsonError::new(msg))
            }
            UpdatingResponseKind::Updated(record) => {
                HttpResponse::Accepted().json(record)
            }
        },
    }
}
