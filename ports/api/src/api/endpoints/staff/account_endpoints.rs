use crate::{
    endpoints::shared::{UrlGroup, UrlScopes},
    modules::{
        AccountFetchingModule, AccountTypeRegistrationModule,
        AccountUpdatingModule,
    },
};

use actix_web::{patch, web, HttpResponse, Responder};
use clean_base::entities::UpdatingResponseKind;
use myc_core::{
    domain::{
        dtos::account::AccountTypeEnum,
        entities::{AccountFetching, AccountTypeRegistration, AccountUpdating},
    },
    use_cases::roles::staff::account::{
        downgrade_account_privileges, upgrade_account_privileges,
    },
};
use myc_http_tools::{middleware::MyceliumProfileData, utils::JsonError};
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
    pub target_account_type: AccountTypeEnum,
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
    context_path = UrlGroup::Accounts.with_scope(UrlScopes::Staffs),
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
    account_fetching_repo: Inject<AccountFetchingModule, dyn AccountFetching>,
    account_updating_repo: Inject<AccountUpdatingModule, dyn AccountUpdating>,
    account_type_registration_repo: Inject<
        AccountTypeRegistrationModule,
        dyn AccountTypeRegistration,
    >,
) -> impl Responder {
    match upgrade_account_privileges(
        profile.to_profile(),
        path.to_owned(),
        info.target_account_type.to_owned(),
        Box::new(&*account_fetching_repo),
        Box::new(&*account_updating_repo),
        Box::new(&*account_type_registration_repo),
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

/// Downgrade account privileges
///
/// Decrease permissions of the refereed account.
#[utoipa::path(
    patch,
    context_path = UrlGroup::Accounts.with_scope(UrlScopes::Staffs),
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
    account_fetching_repo: Inject<AccountFetchingModule, dyn AccountFetching>,
    account_updating_repo: Inject<AccountUpdatingModule, dyn AccountUpdating>,
    account_type_registration_repo: Inject<
        AccountTypeRegistrationModule,
        dyn AccountTypeRegistration,
    >,
) -> impl Responder {
    match downgrade_account_privileges(
        profile.to_profile(),
        path.to_owned(),
        info.target_account_type.to_owned(),
        Box::new(&*account_fetching_repo),
        Box::new(&*account_updating_repo),
        Box::new(&*account_type_registration_repo),
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
