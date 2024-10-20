use crate::{
    dtos::MyceliumProfileData,
    endpoints::{
        shared::{PaginationParams, UrlGroup},
        standard::shared::build_actor_context,
    },
    modules::{
        AccountFetchingModule, AccountRegistrationModule,
        AccountUpdatingModule, WebHookFetchingModule,
    },
};

use actix_web::{get, post, web, HttpResponse, Responder};
use actix_web_httpauth::extractors::bearer::BearerAuth;
use myc_core::{
    domain::{
        actors::ActorName,
        dtos::{account::VerboseStatus, native_error_codes::NativeErrorCodes},
        entities::{
            AccountFetching, AccountRegistration, AccountUpdating,
            WebHookFetching,
        },
    },
    use_cases::roles::standard::subscription_manager::account::{
        create_subscription_account, get_account_details,
        list_accounts_by_type, propagate_existing_subscription_account,
        update_account_name_and_flags,
    },
};
use myc_http_tools::{
    utils::HttpJsonResponse,
    wrappers::default_response_to_http_response::{
        fetch_many_response_kind, fetch_response_kind, updating_response_kind,
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
        .service(create_subscription_account_url)
        .service(update_account_name_and_flags_url)
        .service(list_accounts_by_type_url)
        .service(get_account_details_url)
        .service(propagate_existing_subscription_account_url);
}

// ? ---------------------------------------------------------------------------
// ? Define API structs
// ? ---------------------------------------------------------------------------

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateSubscriptionAccountBody {
    name: String,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSubscriptionAccountNameAndFlagsBody {
    name: Option<String>,
    is_active: Option<bool>,
    is_checked: Option<bool>,
    is_archived: Option<bool>,
    is_default: Option<bool>,
}

#[derive(Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct ListSubscriptionAccountParams {
    term: Option<String>,
    tag_value: Option<String>,
    is_subscription: Option<bool>,
    is_owner_active: Option<bool>,
    status: Option<VerboseStatus>,
}

// ? ---------------------------------------------------------------------------
// ? Define API paths
//
// Account
//
// ? ---------------------------------------------------------------------------

/// Create Subscription Account
///
/// Subscription accounts represents shared entities, like institutions,
/// groups, but not real persons.
#[utoipa::path(
    post,
    context_path = build_actor_context(ActorName::SubscriptionManager, UrlGroup::Accounts),
    params(
        ("tenant_id" = Uuid, Path, description = "The tenant primary key."),
    ),
    request_body = CreateSubscriptionAccountBody,
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
            description = "Account already exists.",
            body = HttpJsonResponse,
        ),
        (
            status = 201,
            description = "Account created.",
            body = CreateSubscriptionResponse,
        ),
    ),
)]
#[post("/{tenant_id}")]
pub async fn create_subscription_account_url(
    auth: BearerAuth,
    path: web::Path<Uuid>,
    body: web::Json<CreateSubscriptionAccountBody>,
    profile: MyceliumProfileData,
    account_registration_repo: Inject<
        AccountRegistrationModule,
        dyn AccountRegistration,
    >,
    webhook_fetching_repo: Inject<WebHookFetchingModule, dyn WebHookFetching>,
) -> impl Responder {
    match create_subscription_account(
        profile.to_profile(),
        path.into_inner(),
        auth.token().to_owned(),
        body.name.to_owned(),
        Box::new(&*account_registration_repo),
        Box::new(&*webhook_fetching_repo),
    )
    .await
    {
        Err(err) => {
            let code_string = err.code().to_string();

            if err.is_in(vec![
                NativeErrorCodes::MYC00002,
                NativeErrorCodes::MYC00003,
            ]) {
                return HttpResponse::Conflict().json(
                    HttpJsonResponse::new_message(err.to_string())
                        .with_code(code_string),
                );
            }

            HttpResponse::InternalServerError().json(
                HttpJsonResponse::new_message(err.to_string())
                    .with_code(code_string),
            )
        }
        Ok(account) => HttpResponse::Created().json(account),
    }
}

/// List account given an account-type
///
/// Get a filtered (or not) list of accounts.
///
/// List accounts with pagination. The `records` field contains a vector of
/// `Account` model.
///
#[utoipa::path(
    get,
    context_path = build_actor_context(ActorName::SubscriptionManager, UrlGroup::Accounts),
    params(
        ("tenant_id" = Uuid, Path, description = "The tenant primary key."),
        ListSubscriptionAccountParams,
        PaginationParams,
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
            body = [Account],
        ),
    ),
)]
#[get("/{tenant_id}")]
pub async fn list_accounts_by_type_url(
    path: web::Path<Uuid>,
    info: web::Query<ListSubscriptionAccountParams>,
    page: web::Query<PaginationParams>,
    profile: MyceliumProfileData,
    account_fetching_repo: Inject<AccountFetchingModule, dyn AccountFetching>,
) -> impl Responder {
    let mut is_account_active: Option<bool> = None;
    let mut is_account_checked: Option<bool> = None;
    let mut is_account_archived: Option<bool> = None;

    match info.status.to_owned() {
        Some(res) => {
            let flags = match res.to_flags() {
                Err(err) => {
                    return HttpResponse::InternalServerError()
                        .json(HttpJsonResponse::new_message(err.to_string()))
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
        path.into_inner(),
        info.term.to_owned(),
        info.is_owner_active.to_owned(),
        is_account_active,
        is_account_checked,
        is_account_archived,
        info.is_subscription.to_owned(),
        info.tag_value.to_owned(),
        page.page_size.to_owned(),
        page.skip.to_owned(),
        Box::new(&*account_fetching_repo),
    )
    .await
    {
        Ok(res) => fetch_many_response_kind(res),
        Err(err) => HttpResponse::InternalServerError()
            .json(HttpJsonResponse::new_message(err.to_string())),
    }
}

/// Get Subscription Account
///
/// Get a single subscription account.
#[utoipa::path(
    get,
    context_path = build_actor_context(ActorName::SubscriptionManager, UrlGroup::Accounts),
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
            body = Account,
        ),
    ),
)]
#[get("/{tenant_id}/accounts/{account_id}")]
pub async fn get_account_details_url(
    path: web::Path<(Uuid, Uuid)>,
    profile: MyceliumProfileData,
    account_fetching_repo: Inject<AccountFetchingModule, dyn AccountFetching>,
) -> impl Responder {
    let (tenant_id, account_id) = path.into_inner();

    match get_account_details(
        profile.to_profile(),
        tenant_id,
        account_id,
        Box::new(&*account_fetching_repo),
    )
    .await
    {
        Ok(res) => fetch_response_kind(res),
        Err(err) => HttpResponse::InternalServerError()
            .json(HttpJsonResponse::new_message(err.to_string())),
    }
}

/// Create Subscription Account
///
/// Subscription accounts represents shared entities, like institutions,
/// groups, but not real persons.
#[utoipa::path(
    patch,
    context_path = build_actor_context(ActorName::SubscriptionManager, UrlGroup::Accounts),
    params(
        ("tenant_id" = Uuid, Path, description = "The tenant primary key."),
        ("account" = Uuid, Path, description = "The account primary key."),
    ),
    request_body = UpdateSubscriptionAccountNameAndFlagsBody,
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
            description = "Account already exists.",
            body = HttpJsonResponse,
        ),
        (
            status = 201,
            description = "Account created.",
            body = CreateSubscriptionResponse,
        ),
    ),
)]
#[get("/{tenant_id}/accounts/{account_id}")]
pub async fn update_account_name_and_flags_url(
    path: web::Path<(Uuid, Uuid)>,
    body: web::Json<UpdateSubscriptionAccountNameAndFlagsBody>,
    profile: MyceliumProfileData,
    account_fetching_repo: Inject<AccountFetchingModule, dyn AccountFetching>,
    account_updating_repo: Inject<AccountUpdatingModule, dyn AccountUpdating>,
) -> impl Responder {
    let (tenant_id, account_id) = path.into_inner();

    match update_account_name_and_flags(
        profile.to_profile(),
        account_id,
        tenant_id,
        body.name.to_owned(),
        body.is_active.to_owned(),
        body.is_checked.to_owned(),
        body.is_archived.to_owned(),
        body.is_default.to_owned(),
        Box::new(&*account_fetching_repo),
        Box::new(&*account_updating_repo),
    )
    .await
    {
        Ok(res) => updating_response_kind(res),
        Err(err) => HttpResponse::InternalServerError()
            .json(HttpJsonResponse::new_message(err.to_string())),
    }
}

/// Propagate Subscription Account
///
/// Propagate a single subscription account.
#[utoipa::path(
    post,
    context_path = build_actor_context(ActorName::SubscriptionManager, UrlGroup::Accounts),
    params(
        ("tenant_id" = Uuid, Path, description = "The tenant primary key."),
        ("account" = Uuid, Path, description = "The account primary key."),
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
            description = "Propagating success.",
            body = Account,
        ),
    ),
)]
#[post("/{tenant_id}/accounts/{account_id}/propagate")]
pub async fn propagate_existing_subscription_account_url(
    auth: BearerAuth,
    path: web::Path<(Uuid, Uuid)>,
    profile: MyceliumProfileData,
    account_fetching_repo: Inject<AccountFetchingModule, dyn AccountFetching>,
    webhook_fetching_repo: Inject<WebHookFetchingModule, dyn WebHookFetching>,
) -> impl Responder {
    let (tenant_id, account_id) = path.into_inner();

    match propagate_existing_subscription_account(
        profile.to_profile(),
        tenant_id,
        auth.token().to_owned(),
        account_id,
        Box::new(&*account_fetching_repo),
        Box::new(&*webhook_fetching_repo),
    )
    .await
    {
        Ok(res) => HttpResponse::Ok().json(res),
        Err(err) => HttpResponse::InternalServerError()
            .json(HttpJsonResponse::new_message(err.to_string())),
    }
}
