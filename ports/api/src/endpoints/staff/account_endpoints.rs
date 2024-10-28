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
use utoipa::ToSchema;
use uuid::Uuid;

// ? ---------------------------------------------------------------------------
// ? Configure application
// ? ---------------------------------------------------------------------------

pub fn configure(config: &mut web::ServiceConfig) {
    config.service(
        web::scope("/accounts")
            .service(upgrade_account_privileges_url)
            .service(downgrade_account_privileges_url),
    );
}

// ? ---------------------------------------------------------------------------
// ? Define API structs
// ? ---------------------------------------------------------------------------

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
enum UpgradeTargetAccountType {
    Staff,
    Manager,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
enum DowngradeTargetAccountType {
    Manager,
    User,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpgradeAccountPrivilegesBody {
    to: UpgradeTargetAccountType,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DowngradeAccountPrivilegesBody {
    to: DowngradeTargetAccountType,
}

// ? ---------------------------------------------------------------------------
// ? Define API paths
//
// Account
//
// ? ---------------------------------------------------------------------------

/// Upgrade account privileges
///
/// Increase permissions of the refereed account.
#[utoipa::path(
    patch,
    context_path = UrlGroup::Accounts.with_scope(UrlScope::Staffs),
    params(
        ("account" = Uuid, Path, description = "The account primary key."),
    ),
    request_body = UpgradeAccountPrivilegesBody,
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
            description = "Account not upgraded.",
            body = HttpJsonResponse,
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
    body: web::Json<UpgradeAccountPrivilegesBody>,
    profile: MyceliumProfileData,
    account_updating_repo: Inject<AccountUpdatingModule, dyn AccountUpdating>,
) -> impl Responder {
    match upgrade_account_privileges(
        profile.to_profile(),
        path.to_owned(),
        match body.to {
            UpgradeTargetAccountType::Manager => AccountTypeV2::Manager,
            UpgradeTargetAccountType::Staff => AccountTypeV2::Staff,
        },
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
    ),
    request_body = DowngradeAccountPrivilegesBody,
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
            description = "Account not downgraded.",
            body = HttpJsonResponse,
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
    body: web::Json<DowngradeAccountPrivilegesBody>,
    profile: MyceliumProfileData,
    account_updating_repo: Inject<AccountUpdatingModule, dyn AccountUpdating>,
) -> impl Responder {
    match downgrade_account_privileges(
        profile.to_profile(),
        path.to_owned(),
        match body.to {
            DowngradeTargetAccountType::Manager => AccountTypeV2::Manager,
            DowngradeTargetAccountType::User => AccountTypeV2::User,
        },
        Box::new(&*account_updating_repo),
    )
    .await
    {
        Ok(res) => updating_response_kind(res),
        Err(err) => HttpResponse::InternalServerError()
            .json(HttpJsonResponse::new_message(err.to_string())),
    }
}
