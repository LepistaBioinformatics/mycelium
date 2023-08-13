use crate::{
    endpoints::shared::PaginationParams,
    modules::{
        AccountFetchingModule, AccountRegistrationModule,
        AccountTypeRegistrationModule, AccountUpdatingModule,
        WebHookFetchingModule,
    },
};

use actix_web::{get, patch, post, web, HttpResponse, Responder};
use actix_web_httpauth::extractors::bearer::Config;
use clean_base::entities::{
    FetchManyResponseKind, FetchResponseKind, UpdatingResponseKind,
};
use myc_core::{
    domain::{
        dtos::{account::VerboseStatus, native_error_codes::NativeErrorCodes},
        entities::{
            AccountFetching, AccountRegistration, AccountTypeRegistration,
            AccountUpdating, WebHookFetching,
        },
    },
    use_cases::roles::managers::account::{
        change_account_activation_status, change_account_approval_status,
        change_account_archival_status, create_subscription_account,
        get_account_details, list_accounts_by_type,
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
    config.service(
        web::scope("/accounts")
            .app_data(Config::default())
            .service(create_subscription_account_url)
            .service(list_accounts_by_type_url)
            .service(get_account_details_url)
            .service(approve_account_url)
            .service(disapprove_account_url)
            .service(activate_account_url)
            .service(deactivate_account_url)
            .service(archive_account_url)
            .service(unarchive_account_url),
    );
}

// ? -----------------------------------------------------------------------
// ? Define API structs
// ? -----------------------------------------------------------------------

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateSubscriptionAccountBody {
    pub email: String,
    pub account_name: String,
}

#[derive(Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct ListSubscriptionAccountParams {
    term: Option<String>,
    is_subscription: Option<bool>,
    is_owner_active: Option<bool>,
    status: Option<VerboseStatus>,
}

// ? -----------------------------------------------------------------------
// ? Define API paths
//
// Account
//
// ? -----------------------------------------------------------------------

/// Create Subscription Account
///
/// Subscription accounts represents shared entities, like institutions,
/// groups, but not real persons.
#[utoipa::path(
    post,
    context_path = "/myc/managers/accounts",
    request_body = CreateSubscriptionAccountBody,
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
            description = "Account already exists.",
            body = JsonError,
        ),
        (
            status = 201,
            description = "Account created.",
            body = CreateSubscriptionResponse,
        ),
    ),
)]
#[post("/")]
pub async fn create_subscription_account_url(
    body: web::Json<CreateSubscriptionAccountBody>,
    profile: MyceliumProfileData,
    account_type_registration_repo: Inject<
        AccountTypeRegistrationModule,
        dyn AccountTypeRegistration,
    >,
    account_registration_repo: Inject<
        AccountRegistrationModule,
        dyn AccountRegistration,
    >,
    webhook_fetching_repo: Inject<WebHookFetchingModule, dyn WebHookFetching>,
) -> impl Responder {
    match create_subscription_account(
        profile.to_profile(),
        body.email.to_owned(),
        body.account_name.to_owned(),
        Box::new(&*account_type_registration_repo),
        Box::new(&*account_registration_repo),
        Box::new(&*webhook_fetching_repo),
    )
    .await
    {
        Err(err) => {
            let code_string = err.code().to_string();

            if err.is_in(vec![
                NativeErrorCodes::MYC00002.as_str(),
                NativeErrorCodes::MYC00003.as_str(),
            ]) {
                return HttpResponse::BadRequest().json(
                    JsonError::new(err.to_string()).with_code(code_string),
                );
            }

            HttpResponse::InternalServerError()
                .json(JsonError::new(err.to_string()).with_code(code_string))
        }
        Ok(account) => HttpResponse::Created().json(account),
    }
}

/// List account given an account-type
///
/// Get a filtered (or not) list of accounts.
///
/// List accounts with pagination. The `records` field contains a vector of
/// [`Account`] model.
///
#[utoipa::path(
    get,
    context_path = "/myc/managers/accounts",
    params(
        ListSubscriptionAccountParams,
        PaginationParams,
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
            body = [Account],
        ),
    ),
)]
#[get("/")]
pub async fn list_accounts_by_type_url(
    info: web::Query<ListSubscriptionAccountParams>,
    page: web::Query<PaginationParams>,
    profile: MyceliumProfileData,
    account_fetching_repo: Inject<AccountFetchingModule, dyn AccountFetching>,
    account_type_registration_repo: Inject<
        AccountTypeRegistrationModule,
        dyn AccountTypeRegistration,
    >,
) -> impl Responder {
    let mut is_account_active: Option<bool> = None;
    let mut is_account_checked: Option<bool> = None;
    let mut is_account_archived: Option<bool> = None;

    match info.status.to_owned() {
        Some(res) => {
            let flags = match res.to_flags() {
                Err(err) => {
                    return HttpResponse::InternalServerError()
                        .json(JsonError::new(err.to_string()))
                }
                Ok(res) => res,
            };

            is_account_active = flags.is_active;
            is_account_checked = flags.is_checked;
            is_account_archived = flags.is_archived;
        }
        _ => (),
    }

    match list_accounts_by_type(
        profile.to_profile(),
        info.term.to_owned(),
        info.is_owner_active.to_owned(),
        is_account_active,
        is_account_checked,
        is_account_archived,
        info.is_subscription.to_owned(),
        page.page_size.to_owned(),
        page.skip.to_owned(),
        Box::new(&*account_fetching_repo),
        Box::new(&*account_type_registration_repo),
    )
    .await
    {
        Err(err) => HttpResponse::InternalServerError()
            .json(JsonError::new(err.to_string())),
        Ok(res) => match res {
            FetchManyResponseKind::NotFound => {
                HttpResponse::NoContent().finish()
            }
            FetchManyResponseKind::Found(accounts) => {
                HttpResponse::Ok().json(accounts)
            }
            FetchManyResponseKind::FoundPaginated(accounts) => {
                HttpResponse::Ok().json(accounts)
            }
        },
    }
}

/// Get Subscription Account
///
/// Get a single subscription account.
#[utoipa::path(
    get,
    context_path = "/myc/managers/accounts",
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
            body = Account,
        ),
    ),
)]
#[get("/{account}")]
pub async fn get_account_details_url(
    path: web::Path<Uuid>,
    profile: MyceliumProfileData,
    account_fetching_repo: Inject<AccountFetchingModule, dyn AccountFetching>,
) -> impl Responder {
    match get_account_details(
        profile.to_profile(),
        *path,
        Box::new(&*account_fetching_repo),
    )
    .await
    {
        Err(err) => HttpResponse::InternalServerError()
            .json(JsonError::new(err.to_string())),
        Ok(res) => match res {
            FetchResponseKind::NotFound(_) => {
                HttpResponse::NoContent().finish()
            }
            FetchResponseKind::Found(accounts) => {
                HttpResponse::Ok().json(accounts)
            }
        },
    }
}

/// Approve account after creation
///
/// New accounts should be approved after has permissions to perform
/// operation on the system. These endpoint should approve such account.
#[utoipa::path(
    patch,
    context_path = "/myc/managers/accounts",
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
    context_path = "/myc/managers/accounts",
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
    context_path = "/myc/managers/accounts",
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
    context_path = "/myc/managers/accounts",
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
    context_path = "/myc/managers/accounts",
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
    context_path = "/myc/managers/accounts",
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
