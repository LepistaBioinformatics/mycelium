use crate::dtos::{MyceliumProfileData, TenantData};

use actix_web::{delete, web, Responder};
use myc_core::use_cases::role_scoped::tenant_manager::delete_subscription_account;
use myc_diesel::repositories::AppModule;
use myc_http_tools::{
    utils::HttpJsonResponse,
    wrappers::default_response_to_http_response::{
        delete_response_kind, handle_mapped_error,
    },
};
use shaku::HasComponent;
use uuid::Uuid;

// ? ---------------------------------------------------------------------------
// ? Configure application
// ? ---------------------------------------------------------------------------

pub fn configure(config: &mut web::ServiceConfig) {
    config.service(delete_subscription_account_url);
}

// ? ---------------------------------------------------------------------------
// ? Define API paths
// ? ---------------------------------------------------------------------------

/// Delete a subscription account.
#[utoipa::path(
    delete,
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
            description = "Account deleted.",
            body = HttpJsonResponse,
        ),
        (
            status = 204,
            description = "Account deleted.",
        ),
    ),
)]
#[delete("/{account_id}")]
pub async fn delete_subscription_account_url(
    tenant: TenantData,
    path: web::Path<Uuid>,
    profile: MyceliumProfileData,
    app_module: web::Data<AppModule>,
) -> impl Responder {
    let account_id = path.into_inner();

    match delete_subscription_account(
        profile.to_profile(),
        tenant.tenant_id().to_owned(),
        account_id,
        Box::new(&*app_module.resolve_ref()),
    )
    .await
    {
        Ok(res) => delete_response_kind(res),
        Err(err) => handle_mapped_error(err),
    }
}
