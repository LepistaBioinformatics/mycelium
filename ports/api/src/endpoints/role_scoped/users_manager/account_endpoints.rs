use crate::{
    dtos::MyceliumProfileData,
    modules::{AccountFetchingModule, AccountUpdatingModule},
};

use actix_web::{patch, web, Responder};
use myc_core::{
    domain::entities::{AccountFetching, AccountUpdating},
    use_cases::roles::role_scoped::users_manager::account::{
        change_account_activation_status, change_account_approval_status,
        change_account_archival_status,
    },
};
use myc_http_tools::{
    utils::HttpJsonResponse,
    wrappers::default_response_to_http_response::{
        handle_mapped_error, updating_response_kind,
    },
    Account,
};
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
    params(
        ("account_id" = Uuid, Path, description = "The account primary key."),
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
            description = "Account not approved.",
            body = HttpJsonResponse,
        ),
        (
            status = 202,
            description = "Account approved.",
            body = Account,
        ),
    ),
)]
#[patch("/{account_id}/approve")]
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
        Ok(res) => updating_response_kind(res),
        Err(err) => handle_mapped_error(err),
    }
}

/// Disapprove account after creation
///
/// Also approved account should be disapproved at any time. These endpoint
/// work for this.
#[utoipa::path(
    patch,
    params(
        ("account_id" = Uuid, Path, description = "The account primary key."),
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
            description = "Account not disapproved.",
            body = HttpJsonResponse,
        ),
        (
            status = 202,
            description = "Account disapproved.",
            body = Account,
        ),
    ),
)]
#[patch("/{account_id}/disapprove")]
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
        Ok(res) => updating_response_kind(res),
        Err(err) => handle_mapped_error(err),
    }
}

/// Activate account
///
/// Any account could be activated and deactivated. This action turn an
/// account active.
#[utoipa::path(
    patch,
    params(
        ("account_id" = Uuid, Path, description = "The account primary key."),
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
            description = "Account not activated.",
            body = HttpJsonResponse,
        ),
        (
            status = 202,
            description = "Account activated.",
            body = Account,
        ),
    ),
)]
#[patch("/{account_id}/activate")]
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
        Ok(res) => updating_response_kind(res),
        Err(err) => handle_mapped_error(err),
    }
}

/// Deactivate account
///
/// Any account could be activated and deactivated. This action turn an
/// account deactivated.
#[utoipa::path(
    patch,
    params(
        ("account_id" = Uuid, Path, description = "The account primary key."),
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
            description = "Account not activated.",
            body = HttpJsonResponse,
        ),
        (
            status = 202,
            description = "Account activated.",
            body = Account,
        ),
    ),
)]
#[patch("/{account_id}/deactivate")]
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
        Ok(res) => updating_response_kind(res),
        Err(err) => handle_mapped_error(err),
    }
}

/// Archive account
///
/// Set target account as archived.
#[utoipa::path(
    patch,
    params(
        ("account_id" = Uuid, Path, description = "The account primary key."),
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
            description = "Account not activated.",
            body = HttpJsonResponse,
        ),
        (
            status = 202,
            description = "Account activated.",
            body = Account,
        ),
    ),
)]
#[patch("/{account_id}/archive")]
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
        Ok(res) => updating_response_kind(res),
        Err(err) => handle_mapped_error(err),
    }
}

/// Unarchive account
///
/// Set target account as un-archived.
#[utoipa::path(
    patch,
    params(
        ("account_id" = Uuid, Path, description = "The account primary key."),
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
            description = "Account not activated.",
            body = HttpJsonResponse,
        ),
        (
            status = 202,
            description = "Account activated.",
            body = Account,
        ),
    ),
)]
#[patch("/{account_id}/unarchive")]
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
        Ok(res) => updating_response_kind(res),
        Err(err) => handle_mapped_error(err),
    }
}
