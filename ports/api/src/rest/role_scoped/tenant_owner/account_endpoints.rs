use crate::dtos::{MyceliumProfileData, TenantData};

use actix_web::{delete, post, web, Responder};
use myc_core::use_cases::role_scoped::tenant_owner::{
    create_management_account, delete_tenant_manager_account,
};
use myc_diesel::repositories::SqlAppModule;
use myc_http_tools::{
    utils::HttpJsonResponse,
    wrappers::default_response_to_http_response::{
        delete_response_kind, get_or_create_response_kind, handle_mapped_error,
    },
    Account,
};
use shaku::HasComponent;
use uuid::Uuid;

// ? ---------------------------------------------------------------------------
// ? Configure application
// ? ---------------------------------------------------------------------------

pub fn configure(config: &mut web::ServiceConfig) {
    config
        .service(create_management_account_url)
        .service(delete_tenant_manager_account_url);
}

// ? ---------------------------------------------------------------------------
// ? Define API paths
// ? ---------------------------------------------------------------------------

/// Create a management account
///
/// Management accounts are used to manage tenant resources. Tenant managers
/// should manage subscription accounts.
///
#[utoipa::path(
    post,
    operation_id = "create_management_account",
    params(
        (
            "x-mycelium-tenant-id" = Uuid,
            Header,
            description = "The tenant unique id."
        ),
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
pub async fn create_management_account_url(
    tenant: TenantData,
    profile: MyceliumProfileData,
    app_module: web::Data<SqlAppModule>,
) -> impl Responder {
    match create_management_account(
        profile.to_profile(),
        tenant.tenant_id().to_owned(),
        Box::new(&*app_module.resolve_ref()),
    )
    .await
    {
        Ok(res) => get_or_create_response_kind(res),
        Err(err) => handle_mapped_error(err),
    }
}

/// Delete a tenant manager account
///
/// This action will soft delete the tenant manager account.
///
#[utoipa::path(
    delete,
    operation_id = "delete_tenant_manager_account",
    params(
        (
            "x-mycelium-tenant-id" = Uuid,
            Header,
            description = "The tenant unique id."
        ),
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
            status = 200,
            description = "Account deleted.",
            body = HttpJsonResponse,
        ),
    ),
)]
#[delete("/{account_id}")]
pub async fn delete_tenant_manager_account_url(
    tenant: TenantData,
    path: web::Path<Uuid>,
    profile: MyceliumProfileData,
    app_module: web::Data<SqlAppModule>,
) -> impl Responder {
    match delete_tenant_manager_account(
        profile.to_profile(),
        tenant.tenant_id().to_owned(),
        path.to_owned(),
        Box::new(&*app_module.resolve_ref()),
    )
    .await
    {
        Ok(res) => delete_response_kind(res),
        Err(err) => handle_mapped_error(err),
    }
}
