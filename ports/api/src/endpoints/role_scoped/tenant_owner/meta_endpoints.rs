use std::str::FromStr;

use crate::dtos::{MyceliumProfileData, TenantData};

use actix_web::{delete, post, web, HttpResponse, Responder};
use myc_core::{
    domain::dtos::tenant::{TenantMeta, TenantMetaKey},
    use_cases::role_scoped::tenant_owner::{
        create_tenant_meta, delete_tenant_meta,
    },
};
use myc_diesel::repositories::AppModule;
use myc_http_tools::{
    utils::HttpJsonResponse,
    wrappers::default_response_to_http_response::{
        create_response_kind, delete_response_kind, handle_mapped_error,
    },
};
use serde::Deserialize;
use shaku::HasComponent;
use utoipa::ToSchema;

// ? ---------------------------------------------------------------------------
// ? Configure application
// ? ---------------------------------------------------------------------------

pub fn configure(config: &mut web::ServiceConfig) {
    config
        .service(create_tenant_meta_url)
        .service(delete_tenant_meta_url);
}

// ? ---------------------------------------------------------------------------
// ? Define API structs
// ? ---------------------------------------------------------------------------

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateTenantMetaBody {
    key: String,
    value: String,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeleteTenantMetaBody {
    key: String,
}

// ? ---------------------------------------------------------------------------
// ? Define API paths
// ? ---------------------------------------------------------------------------

/// Register a tenant metadata
#[utoipa::path(
    post,
    params(
        (
            "x-mycelium-tenant-id" = Uuid,
            Header,
            description = "The tenant unique id."
        ),
    ),
    request_body = CreateTenantMetaBody,
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
            description = "Meta already exists.",
            body = HttpJsonResponse,
        ),
        (
            status = 201,
            description = "Meta created.",
            body = TenantMeta,
        ),
    ),
)]
#[post("")]
pub async fn create_tenant_meta_url(
    tenant: TenantData,
    body: web::Json<CreateTenantMetaBody>,
    profile: MyceliumProfileData,
    app_module: web::Data<AppModule>,
) -> impl Responder {
    let key = match TenantMetaKey::from_str(&body.key) {
        Ok(key) => key,
        Err(_) => {
            return HttpResponse::BadRequest().json(
                HttpJsonResponse::new_message("The key is invalid".to_string()),
            );
        }
    };

    match create_tenant_meta(
        profile.to_profile(),
        tenant.tenant_id().to_owned(),
        key.to_owned(),
        body.value.to_owned(),
        Box::new(&*app_module.resolve_ref()),
    )
    .await
    {
        Ok(res) => create_response_kind(res),
        Err(err) => handle_mapped_error(err),
    }
}

/// Delete a tenant metadata
#[utoipa::path(
    delete,
    params(
        (
            "x-mycelium-tenant-id" = Uuid,
            Header,
            description = "The tenant unique id."
        ),
    ),
    request_body = DeleteTenantMetaBody,
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
            description = "Meta not deleted.",
            body = HttpJsonResponse,
        ),
        (
            status = 204,
            description = "Meta deleted.",
        ),
    ),
)]
#[delete("")]
pub async fn delete_tenant_meta_url(
    tenant: TenantData,
    body: web::Json<DeleteTenantMetaBody>,
    profile: MyceliumProfileData,
    app_module: web::Data<AppModule>,
) -> impl Responder {
    let key = match TenantMetaKey::from_str(&body.key) {
        Ok(key) => key,
        Err(_) => {
            return HttpResponse::BadRequest().json(
                HttpJsonResponse::new_message("The key is invalid".to_string()),
            );
        }
    };

    match delete_tenant_meta(
        profile.to_profile(),
        tenant.tenant_id().to_owned(),
        key.to_owned(),
        Box::new(&*app_module.resolve_ref()),
    )
    .await
    {
        Ok(res) => delete_response_kind(res),
        Err(err) => handle_mapped_error(err),
    }
}
