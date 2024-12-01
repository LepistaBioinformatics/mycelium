use crate::{
    dtos::{MyceliumProfileData, TenantData},
    endpoints::shared::PaginationParams,
    modules::{
        AccountFetchingModule, AccountRegistrationModule,
        AccountUpdatingModule, WebHookFetchingModule,
    },
};

use actix_web::{get, patch, post, web, HttpResponse, Responder};
use myc_core::{
    domain::{
        actors::ActorName,
        dtos::{account::VerboseStatus, account_type::AccountTypeV2},
        entities::{
            AccountFetching, AccountRegistration, AccountUpdating,
            WebHookFetching,
        },
    },
    use_cases::roles::standard::subscriptions_manager::account::{
        create_subscription_account, get_account_details,
        list_accounts_by_type, propagate_existing_subscription_account,
        update_account_name_and_flags,
    },
};
use myc_http_tools::{
    utils::HttpJsonResponse,
    wrappers::default_response_to_http_response::{
        fetch_many_response_kind, fetch_response_kind, handle_mapped_error,
        updating_response_kind,
    },
    Account,
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

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) enum APIAccountType {
    Staff,
    Manager,
    User,
    Subscription,
    StandardRoleAssociated,
    TenantManager,
}

impl APIAccountType {
    fn into_account_type_v2(
        &self,
        tenant_id: Uuid,
        role_name: Option<String>,
        role_id: Option<Uuid>,
    ) -> Result<AccountTypeV2, HttpResponse> {
        match self {
            APIAccountType::Staff => Ok(AccountTypeV2::Staff),
            APIAccountType::Manager => Ok(AccountTypeV2::Manager),
            APIAccountType::User => Ok(AccountTypeV2::User),
            APIAccountType::TenantManager => Ok(AccountTypeV2::TenantManager {
                tenant_id: tenant_id.to_owned(),
            }),
            APIAccountType::Subscription => Ok(AccountTypeV2::Subscription {
                tenant_id: tenant_id.to_owned(),
            }),
            APIAccountType::StandardRoleAssociated => {
                if role_name.is_none() || role_id.is_none() {
                    return Err(HttpResponse::BadRequest().json(
                        HttpJsonResponse::new_message(
                            "Role name and role id are required.",
                        ),
                    ));
                }

                Ok(AccountTypeV2::StandardRoleAssociated {
                    tenant_id: tenant_id.to_owned(),
                    role_name: ActorName::CustomRole(role_name.unwrap()),
                    role_id: role_id.unwrap(),
                })
            }
        }
    }
}

#[derive(Deserialize, ToSchema, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct ListSubscriptionAccountParams {
    term: Option<String>,
    tag_value: Option<String>,
    account_type: Option<APIAccountType>,
    is_owner_active: Option<bool>,
    status: Option<VerboseStatus>,
    role_name: Option<String>,
    role_id: Option<Uuid>,
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
    params(
        (
            "x-mycelium-tenant-id" = Uuid,
            Header,
            description = "The tenant unique id."
        ),
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
            body = Account,
        ),
    ),
)]
#[post("/")]
pub async fn create_subscription_account_url(
    tenant: TenantData,
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
        tenant.tenant_id().to_owned(),
        body.name.to_owned(),
        Box::new(&*account_registration_repo),
        Box::new(&*webhook_fetching_repo),
    )
    .await
    {
        Err(err) => handle_mapped_error(err),
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
    params(
        (
            "x-mycelium-tenant-id" = Uuid,
            Header,
            description = "The tenant unique id."
        ),
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
#[get("/")]
pub async fn list_accounts_by_type_url(
    tenant: TenantData,
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

    let tenant_id = tenant.tenant_id().to_owned();

    let account_type = match &info.account_type {
        None => None,
        Some(res) => match res.into_account_type_v2(
            tenant_id,
            info.role_name.to_owned(),
            info.role_id,
        ) {
            Ok(res) => Some(res),
            Err(err) => return err,
        },
    };

    match list_accounts_by_type(
        profile.to_profile(),
        tenant_id.to_owned(),
        info.term.to_owned(),
        info.is_owner_active.to_owned(),
        is_account_active,
        is_account_checked,
        is_account_archived,
        account_type,
        info.tag_value.to_owned(),
        page.page_size.to_owned(),
        page.skip.to_owned(),
        Box::new(&*account_fetching_repo),
    )
    .await
    {
        Ok(res) => fetch_many_response_kind(res),
        Err(err) => handle_mapped_error(err),
    }
}

/// Get Subscription Account
///
/// Get a single subscription account.
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
            body = Account,
        ),
    ),
)]
#[get("/{account_id}")]
pub async fn get_account_details_url(
    tenant: TenantData,
    path: web::Path<Uuid>,
    profile: MyceliumProfileData,
    account_fetching_repo: Inject<AccountFetchingModule, dyn AccountFetching>,
) -> impl Responder {
    let account_id = path.into_inner();

    match get_account_details(
        profile.to_profile(),
        tenant.tenant_id().to_owned(),
        account_id,
        Box::new(&*account_fetching_repo),
    )
    .await
    {
        Ok(res) => fetch_response_kind(res),
        Err(err) => handle_mapped_error(err),
    }
}

/// Create Subscription Account
///
/// Subscription accounts represents shared entities, like institutions,
/// groups, but not real persons.
#[utoipa::path(
    patch,
    params(
        (
            "x-mycelium-tenant-id" = Uuid,
            Header,
            description = "The tenant unique id."
        ),
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
            body = Account,
        ),
    ),
)]
#[patch("/{account_id}")]
pub async fn update_account_name_and_flags_url(
    tenant: TenantData,
    path: web::Path<Uuid>,
    body: web::Json<UpdateSubscriptionAccountNameAndFlagsBody>,
    profile: MyceliumProfileData,
    account_fetching_repo: Inject<AccountFetchingModule, dyn AccountFetching>,
    account_updating_repo: Inject<AccountUpdatingModule, dyn AccountUpdating>,
) -> impl Responder {
    let account_id = path.into_inner();

    match update_account_name_and_flags(
        profile.to_profile(),
        account_id,
        tenant.tenant_id().to_owned(),
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
        Err(err) => handle_mapped_error(err),
    }
}

/// Propagate Subscription Account
///
/// Propagate a single subscription account.
#[utoipa::path(
    post,
    params(
        (
            "x-mycelium-tenant-id" = Uuid,
            Header,
            description = "The tenant unique id."
        ),
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
#[post("/{account_id}/propagate")]
pub async fn propagate_existing_subscription_account_url(
    tenant: TenantData,
    path: web::Path<Uuid>,
    profile: MyceliumProfileData,
    account_fetching_repo: Inject<AccountFetchingModule, dyn AccountFetching>,
    webhook_fetching_repo: Inject<WebHookFetchingModule, dyn WebHookFetching>,
) -> impl Responder {
    let account_id = path.into_inner();

    match propagate_existing_subscription_account(
        profile.to_profile(),
        tenant.tenant_id().to_owned(),
        account_id,
        Box::new(&*account_fetching_repo),
        Box::new(&*webhook_fetching_repo),
    )
    .await
    {
        Ok(res) => HttpResponse::Ok().json(res),
        Err(err) => handle_mapped_error(err),
    }
}
