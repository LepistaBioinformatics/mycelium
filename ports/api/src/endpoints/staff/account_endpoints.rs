use crate::{
    dtos::MyceliumProfileData,
    endpoints::shared::{UrlGroup, UrlScope},
    modules::AccountUpdatingModule,
};

use actix_web::{patch, web, HttpResponse, Responder};
use myc_core::{
    domain::{dtos::account_type::AccountTypeV2, entities::AccountUpdating},
    use_cases::roles::staff::account::{
        downgrade_account_privileges, upgrade_account_privileges,
    },
};
use myc_http_tools::{
    utils::HttpJsonResponse,
    wrappers::default_response_to_http_response::updating_response_kind,
};
use serde::Deserialize;
use shaku_actix::Inject;
use utoipa::IntoParams;
use uuid::Uuid;

// ? -----------------------------------------------------------------------
// ? Configure application
// ? -----------------------------------------------------------------------

pub fn configure(config: &mut web::ServiceConfig) {
    config.service(
        web::scope("/accounts")
            .service(upgrade_account_privileges_url)
            .service(downgrade_account_privileges_url),
    );
}

// ? -----------------------------------------------------------------------
// ? Define API structs
// ? -----------------------------------------------------------------------

#[derive(Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct UpgradeAccountPrivilegesParams {
    pub target_account_type: AccountTypeV2,
}

// ? -----------------------------------------------------------------------
// ? Define API paths
//
// Account
//
// ? -----------------------------------------------------------------------

/// Upgrade account privileges
///
/// Increase permissions of the refereed account.
#[utoipa::path(
    patch,
    context_path = UrlGroup::Accounts.with_scope(UrlScope::Staffs),
    params(
        ("account" = Uuid, Path, description = "The account primary key."),
        UpgradeAccountPrivilegesParams,
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
            description = "Account not upgraded.",
            body = JsonError,
        ),
        (
            status = 202,
            description = "Account upgraded.",
            body = Account,
        ),
    ),
)]
#[patch("/{account}/upgrade")]
pub async fn upgrade_account_privileges_url(
    path: web::Path<Uuid>,
    info: web::Query<UpgradeAccountPrivilegesParams>,
    profile: MyceliumProfileData,
    account_updating_repo: Inject<AccountUpdatingModule, dyn AccountUpdating>,
) -> impl Responder {
    match upgrade_account_privileges(
        profile.to_profile(),
        path.to_owned(),
        info.target_account_type.to_owned(),
        Box::new(&*account_updating_repo),
    )
    .await
    {
        Ok(res) => updating_response_kind(res),
        Err(err) => HttpResponse::InternalServerError()
            .json(HttpJsonResponse::new_message(err.to_string())),
    }
}

/// Downgrade account privileges
///
/// Decrease permissions of the refereed account.
#[utoipa::path(
    patch,
    context_path = UrlGroup::Accounts.with_scope(UrlScope::Staffs),
    params(
        ("account" = Uuid, Path, description = "The account primary key."),
        UpgradeAccountPrivilegesParams,
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
            description = "Account not downgraded.",
            body = JsonError,
        ),
        (
            status = 202,
            description = "Account downgraded.",
            body = Account,
        ),
    ),
)]
#[patch("/{account}/downgrade")]
pub async fn downgrade_account_privileges_url(
    path: web::Path<Uuid>,
    info: web::Query<UpgradeAccountPrivilegesParams>,
    profile: MyceliumProfileData,
    account_updating_repo: Inject<AccountUpdatingModule, dyn AccountUpdating>,
) -> impl Responder {
    match downgrade_account_privileges(
        profile.to_profile(),
        path.to_owned(),
        info.target_account_type.to_owned(),
        Box::new(&*account_updating_repo),
    )
    .await
    {
        Ok(res) => updating_response_kind(res),
        Err(err) => HttpResponse::InternalServerError()
            .json(HttpJsonResponse::new_message(err.to_string())),
    }
}
