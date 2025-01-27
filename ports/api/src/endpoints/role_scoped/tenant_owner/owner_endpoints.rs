use crate::dtos::{MyceliumProfileData, TenantData};

use actix_web::{delete, post, web, HttpResponse, Responder};
use myc_core::{
    domain::entities::TenantOwnerConnection,
    use_cases::role_scoped::tenant_owner::{
        guest_tenant_owner, revoke_tenant_owner,
    },
};
use myc_diesel::repositories::SqlAppModule;
use myc_http_tools::{
    utils::HttpJsonResponse,
    wrappers::default_response_to_http_response::{
        create_response_kind, delete_response_kind, handle_mapped_error,
    },
    Email,
};
use serde::Deserialize;
use shaku::HasComponent;
use utoipa::ToSchema;

// ? ---------------------------------------------------------------------------
// ? Configure application
// ? ---------------------------------------------------------------------------

pub fn configure(config: &mut web::ServiceConfig) {
    config
        .service(guest_tenant_owner_url)
        .service(revoke_tenant_owner_url);
}

// ? ---------------------------------------------------------------------------
// ? Define API structs
// ? ---------------------------------------------------------------------------

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GuestTenantOwnerBody {
    email: String,
}

// ? ---------------------------------------------------------------------------
// ? Define API paths
// ? ---------------------------------------------------------------------------

/// Guest a user to work as a tenant owner
#[utoipa::path(
    post,
    params(
        (
            "x-mycelium-tenant-id" = Uuid,
            Header,
            description = "The tenant unique id."
        ),
    ),
    request_body = GuestTenantOwnerBody,
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
            description = "Owner already exists.",
            body = HttpJsonResponse,
        ),
        (
            status = 201,
            description = "Owner created.",
            body = TenantOwnerConnection,
        ),
    ),
)]
#[post("")]
pub async fn guest_tenant_owner_url(
    tenant: TenantData,
    body: web::Json<GuestTenantOwnerBody>,
    profile: MyceliumProfileData,
    app_module: web::Data<SqlAppModule>,
) -> impl Responder {
    let email = match Email::from_string(body.email.to_owned()) {
        Ok(email) => email,
        Err(err) => {
            return HttpResponse::BadRequest()
                .json(HttpJsonResponse::new_message(err.to_string()))
        }
    };

    match guest_tenant_owner(
        profile.to_profile(),
        email,
        tenant.tenant_id().to_owned(),
        Box::new(&*app_module.resolve_ref()),
        Box::new(&*app_module.resolve_ref()),
    )
    .await
    {
        Ok(res) => create_response_kind(res),
        Err(err) => handle_mapped_error(err),
    }
}

/// Revoke a user from working as a tenant owner
#[utoipa::path(
    delete,
    params(
        (
            "x-mycelium-tenant-id" = Uuid,
            Header,
            description = "The tenant unique id."
        ),
    ),
    request_body = GuestTenantOwnerBody,
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
            description = "Owner deleted.",
            body = HttpJsonResponse,
        ),
        (
            status = 204,
            description = "Owner deleted.",
        ),
    ),
)]
#[delete("")]
pub async fn revoke_tenant_owner_url(
    tenant: TenantData,
    body: web::Json<GuestTenantOwnerBody>,
    profile: MyceliumProfileData,
    app_module: web::Data<SqlAppModule>,
) -> impl Responder {
    let email = match Email::from_string(body.email.to_owned()) {
        Ok(email) => email,
        Err(err) => {
            return HttpResponse::BadRequest()
                .json(HttpJsonResponse::new_message(err.to_string()))
        }
    };

    match revoke_tenant_owner(
        profile.to_profile(),
        email,
        tenant.tenant_id().to_owned(),
        Box::new(&*app_module.resolve_ref()),
        Box::new(&*app_module.resolve_ref()),
    )
    .await
    {
        Ok(res) => delete_response_kind(res),
        Err(err) => handle_mapped_error(err),
    }
}
