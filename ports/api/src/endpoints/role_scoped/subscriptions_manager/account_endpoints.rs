use crate::{
    dtos::{MyceliumProfileData, TenantData},
    endpoints::shared::PaginationParams,
};

use actix_web::{get, patch, post, web, HttpResponse, Responder};
use myc_core::{
    domain::{
        actors::SystemActor,
        dtos::{account::VerboseStatus, account_type::AccountType},
    },
    use_cases::role_scoped::subscriptions_manager::account::{
        create_role_associated_account, create_subscription_account,
        get_account_details, list_accounts_by_type,
        propagate_existing_subscription_account, update_account_name_and_flags,
    },
};
use myc_diesel::repositories::SqlAppModule;
use myc_http_tools::{
    utils::HttpJsonResponse,
    wrappers::default_response_to_http_response::{
        fetch_many_response_kind, fetch_response_kind, handle_mapped_error,
        updating_response_kind,
    },
    Account,
};
use mycelium_base::entities::GetOrCreateResponseKind;
use serde::Deserialize;
use shaku::HasComponent;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

// ? ---------------------------------------------------------------------------
// ? Configure application
// ? ---------------------------------------------------------------------------

pub fn configure(config: &mut web::ServiceConfig) {
    config
        .service(create_subscription_account_url)
        .service(create_role_associated_account_url)
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
pub struct CreateRoleAssociatedAccountBody {
    account_name: String,
    role_name: String,
    role_description: String,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSubscriptionAccountNameAndFlagsBody {
    name: Option<String>,
    is_active: Option<bool>,
    is_checked: Option<bool>,
    is_archived: Option<bool>,
    is_system_account: Option<bool>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) enum APIAccountType {
    Staff,
    Manager,
    User,
    Subscription,
    ActorAssociated,
    TenantManager,
    RoleAssociated,
}

impl APIAccountType {
    fn into_account_type(
        &self,
        tenant_id: Option<Uuid>,
        actor: Option<SystemActor>,
    ) -> Result<AccountType, HttpResponse> {
        match self {
            //
            // User related accounts. Not tenant dependent.
            //
            APIAccountType::Staff => Ok(AccountType::Staff),
            APIAccountType::Manager => Ok(AccountType::Manager),
            APIAccountType::User => Ok(AccountType::User),

            //
            // Tenant related accounts. Tenant dependent.
            //
            APIAccountType::TenantManager => {
                if let Some(tenant_id) = tenant_id {
                    Ok(AccountType::TenantManager { tenant_id })
                } else {
                    Err(HttpResponse::BadRequest()
                        .json(HttpJsonResponse::new_message(
                        "Tenant ID is required for tenant manager accounts.",
                    )))
                }
            }
            APIAccountType::Subscription => {
                if let Some(tenant_id) = tenant_id {
                    Ok(AccountType::Subscription { tenant_id })
                } else {
                    Err(HttpResponse::BadRequest().json(
                        HttpJsonResponse::new_message(
                            "Tenant ID is required for subscription accounts.",
                        ),
                    ))
                }
            }

            //
            // Actor related accounts
            //
            APIAccountType::ActorAssociated => {
                if actor.is_none() {
                    return Err(HttpResponse::BadRequest().json(
                        HttpJsonResponse::new_message("Actor is required."),
                    ));
                }

                Ok(AccountType::ActorAssociated {
                    actor: actor.unwrap(),
                })
            }

            //
            // Role associated accounts
            //
            APIAccountType::RoleAssociated => {
                if let Some(tenant_id) = tenant_id {
                    Ok(AccountType::RoleAssociated {
                        tenant_id,
                        role_name: String::new(),
                        read_role_id: Uuid::nil(),
                        write_role_id: Uuid::nil(),
                    })
                } else {
                    Err(HttpResponse::BadRequest().json(
                        HttpJsonResponse::new_message(
                            "Tenant ID is required for subscription accounts.",
                        ),
                    ))
                }
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
    actor: Option<SystemActor>,
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
    operation_id = "create_subscription_account",
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
#[post("")]
pub async fn create_subscription_account_url(
    tenant: TenantData,
    body: web::Json<CreateSubscriptionAccountBody>,
    profile: MyceliumProfileData,
    app_module: web::Data<SqlAppModule>,
) -> impl Responder {
    match create_subscription_account(
        profile.to_profile(),
        tenant.tenant_id().to_owned(),
        body.name.to_owned(),
        Box::new(&*app_module.resolve_ref()),
        Box::new(&*app_module.resolve_ref()),
    )
    .await
    {
        Err(err) => handle_mapped_error(err),
        Ok(account) => HttpResponse::Created().json(account),
    }
}

/// Create Role Associated Account
///
/// Role associated accounts mirrors the guest-roles used to connect peoples in
/// non-personal accounts, like institutions, groups, etc.
#[utoipa::path(
    post,
    operation_id = "create_role_associated_account",
    params(
        (
            "x-mycelium-tenant-id" = Uuid,
            Header,
            description = "The tenant unique id."
        ),
    ),
    request_body = CreateRoleAssociatedAccountBody,
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
#[post("/role-associated")]
pub async fn create_role_associated_account_url(
    tenant: TenantData,
    body: web::Json<CreateRoleAssociatedAccountBody>,
    profile: MyceliumProfileData,
    app_module: web::Data<SqlAppModule>,
) -> impl Responder {
    match create_role_associated_account(
        profile.to_profile(),
        tenant.tenant_id().to_owned(),
        body.account_name.to_owned(),
        body.role_name.to_owned(),
        body.role_description.to_owned(),
        Box::new(&*app_module.resolve_ref()),
        Box::new(&*app_module.resolve_ref()),
    )
    .await
    {
        Err(err) => handle_mapped_error(err),
        Ok(account) => match account {
            GetOrCreateResponseKind::Created(account) => {
                HttpResponse::Created().json(account)
            }
            GetOrCreateResponseKind::NotCreated(account, msg) => {
                tracing::warn!("{}", msg);

                HttpResponse::Ok().json(account)
            }
        },
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
    operation_id = "list_accounts_by_type",
    params(
        (
            "x-mycelium-tenant-id" = Uuid,
            Header,
            description = "The tenant unique id.",
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
#[get("")]
pub async fn list_accounts_by_type_url(
    tenant: Option<TenantData>,
    query: web::Query<ListSubscriptionAccountParams>,
    page: web::Query<PaginationParams>,
    profile: MyceliumProfileData,
    app_module: web::Data<SqlAppModule>,
) -> impl Responder {
    let mut is_account_active: Option<bool> = None;
    let mut is_account_checked: Option<bool> = None;
    let mut is_account_archived: Option<bool> = None;
    let mut is_account_deleted: Option<bool> = None;

    match query.status.to_owned() {
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
            is_account_deleted = flags.is_deleted;
        }
        _ => (),
    }

    let tenant_id = match tenant {
        Some(tenant) => Some(tenant.tenant_id().to_owned()),
        None => None,
    };

    let account_type = match &query.account_type {
        None => None,
        Some(res) => {
            match res.into_account_type(tenant_id, query.actor.to_owned()) {
                Ok(res) => Some(res),
                Err(err) => return err,
            }
        }
    };

    match list_accounts_by_type(
        profile.to_profile(),
        tenant_id.to_owned(),
        query.term.to_owned(),
        query.is_owner_active.to_owned(),
        is_account_active,
        is_account_checked,
        is_account_archived,
        is_account_deleted,
        account_type,
        query.tag_value.to_owned(),
        page.page_size.to_owned(),
        page.skip.to_owned(),
        Box::new(&*app_module.resolve_ref()),
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
    operation_id = "get_account_details",
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
    tenant: Option<TenantData>,
    path: web::Path<Uuid>,
    profile: MyceliumProfileData,
    app_module: web::Data<SqlAppModule>,
) -> impl Responder {
    let account_id = path.into_inner();

    match get_account_details(
        profile.to_profile(),
        tenant.map(|t| t.tenant_id().to_owned()),
        account_id,
        Box::new(&*app_module.resolve_ref()),
    )
    .await
    {
        Ok(res) => fetch_response_kind(res),
        Err(err) => handle_mapped_error(err),
    }
}

/// Update Subscription Account Name and Flags
///
/// Subscription accounts represents shared entities, like institutions,
/// groups, but not real persons.
#[utoipa::path(
    patch,
    operation_id = "update_account_name_and_flags",
    params(
        (
            "x-mycelium-tenant-id" = Uuid,
            Header,
            description = "The tenant unique id."
        ),
        ("account_id" = Uuid, Path, description = "The account primary key."),
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
    app_module: web::Data<SqlAppModule>,
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
        body.is_system_account.to_owned(),
        Box::new(&*app_module.resolve_ref()),
        Box::new(&*app_module.resolve_ref()),
        Box::new(&*app_module.resolve_ref()),
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
    operation_id = "propagate_existing_subscription_account",
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
    app_module: web::Data<SqlAppModule>,
) -> impl Responder {
    let account_id = path.into_inner();

    match propagate_existing_subscription_account(
        profile.to_profile(),
        tenant.tenant_id().to_owned(),
        account_id,
        Box::new(&*app_module.resolve_ref()),
        Box::new(&*app_module.resolve_ref()),
    )
    .await
    {
        Ok(res) => HttpResponse::Ok().json(res),
        Err(err) => handle_mapped_error(err),
    }
}
