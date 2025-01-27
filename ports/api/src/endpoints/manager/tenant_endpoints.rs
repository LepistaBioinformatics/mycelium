use std::str::FromStr;

use crate::{dtos::MyceliumProfileData, endpoints::shared::PaginationParams};

use actix_web::{delete, get, patch, post, web, HttpResponse, Responder};
use myc_core::{
    domain::{
        dtos::tenant::{Tenant, TenantMetaKey},
        entities::TenantOwnerConnection,
    },
    use_cases::super_users::managers::{
        create_tenant, delete_tenant, exclude_tenant_owner,
        include_tenant_owner, list_tenant,
    },
};
use myc_diesel::repositories::SqlAppModule;
use myc_http_tools::{
    utils::HttpJsonResponse,
    wrappers::default_response_to_http_response::{
        create_response_kind, delete_response_kind, fetch_many_response_kind,
        handle_mapped_error,
    },
};
use serde::Deserialize;
use shaku::HasComponent;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

// ? ---------------------------------------------------------------------------
// ? Configure application
// ? ---------------------------------------------------------------------------

pub fn configure(config: &mut web::ServiceConfig) {
    config.service(
        web::scope("/tenants")
            .service(create_tenant_url)
            .service(list_tenant_url)
            .service(delete_tenant_url)
            .service(include_tenant_owner_url)
            .service(exclude_tenant_owner_url),
    );
}

// ? ---------------------------------------------------------------------------
// ? Define API structs
// ? ---------------------------------------------------------------------------

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateTenantBody {
    /// The name of the tenant
    name: String,

    /// The description of the tenant
    description: Option<String>,

    /// The owner of the tenant
    owner_id: Uuid,
}

#[derive(Deserialize, ToSchema, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct ListTenantParams {
    /// Filter tenants by name
    name: Option<String>,

    /// Filter tenants by owner
    owner: Option<Uuid>,

    /// Filter tenants by metadata key
    metadata: Option<String>,

    /// Filter tenants by tag
    ///
    /// Example: `key=value`
    tag: Option<String>,
}

// ? ---------------------------------------------------------------------------
// ? Define API paths
// ? ---------------------------------------------------------------------------

/// Create a new tenant
#[utoipa::path(
    post,
    request_body = CreateTenantBody,
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
            description = "Tenant already exists.",
            body = HttpJsonResponse,
        ),
        (
            status = 201,
            description = "Tenant created.",
            body = Tenant,
        ),
    ),
)]
#[post("")]
pub async fn create_tenant_url(
    body: web::Json<CreateTenantBody>,
    profile: MyceliumProfileData,
    app_module: web::Data<SqlAppModule>,
) -> impl Responder {
    match create_tenant(
        profile.to_profile(),
        body.name.clone(),
        body.description.clone(),
        body.owner_id,
        Box::new(&*app_module.resolve_ref()),
        Box::new(&*app_module.resolve_ref()),
    )
    .await
    {
        Ok(res) => create_response_kind(res),
        Err(err) => handle_mapped_error(err),
    }
}

/// List tenants
#[utoipa::path(
    get,
    params(
        ListTenantParams,
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
            body = [Tenant],
        ),
    ),
)]
#[get("")]
pub async fn list_tenant_url(
    query: web::Query<ListTenantParams>,
    page: web::Query<PaginationParams>,
    profile: MyceliumProfileData,
    app_module: web::Data<SqlAppModule>,
) -> impl Responder {
    let tag = match query.tag.as_ref() {
        Some(tag) => match tag.split_once('=') {
            Some((key, value)) => Some((key.to_string(), value.to_string())),
            None => {
                return HttpResponse::BadRequest().body("Invalid tag format")
            }
        },
        None => None,
    };

    let metadata = match query.metadata.as_ref() {
        Some(metadata) => match metadata.split_once('=') {
            Some((key, value)) => Some((
                match TenantMetaKey::from_str(key) {
                    Ok(key) => key,
                    Err(_) => {
                        return HttpResponse::BadRequest()
                            .body("Invalid metadata key")
                    }
                },
                value.to_string(),
            )),
            None => {
                return HttpResponse::BadRequest()
                    .body("Invalid metadata format")
            }
        },
        None => None,
    };

    match list_tenant(
        profile.to_profile(),
        query.name.to_owned(),
        query.owner.to_owned(),
        metadata.to_owned(),
        tag.to_owned(),
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

/// Delete a tenant
#[utoipa::path(
    delete,
    params(
        ("id" = Uuid, Path, description = "The tenant primary key."),
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
            description = "Bad request.",
            body = HttpJsonResponse,
        ),
        (
            status = 201,
            description = "Tenant successfully registered.",
            body = Uuid,
        ),
    ),
)]
#[delete("/{id}")]
pub async fn delete_tenant_url(
    profile: MyceliumProfileData,
    path: web::Path<Uuid>,
    app_module: web::Data<SqlAppModule>,
) -> impl Responder {
    match delete_tenant(
        profile.to_profile(),
        path.into_inner(),
        Box::new(&*app_module.resolve_ref()),
    )
    .await
    {
        Ok(res) => delete_response_kind(res),
        Err(err) => handle_mapped_error(err),
    }
}

/// Include a tenant owner
///
/// A single tenant can have multiple owners. This endpoint allows to include a
/// new owner to the tenant.
///
#[utoipa::path(
    patch,
    params(
        ("id" = Uuid, Path, description = "The tenant primary key."),
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
            status = 201,
            description = "Owner included.",
            body = TenantOwnerConnection,
        ),
    ),
)]
#[patch("/{id}/owner/{owner_id}")]
pub async fn include_tenant_owner_url(
    path: web::Path<(Uuid, Uuid)>,
    profile: MyceliumProfileData,
    app_module: web::Data<SqlAppModule>,
) -> impl Responder {
    let (tenant_id, owner_id) = path.into_inner();

    match include_tenant_owner(
        profile.to_profile(),
        tenant_id,
        owner_id,
        Box::new(&*app_module.resolve_ref()),
    )
    .await
    {
        Ok(res) => create_response_kind(res),
        Err(err) => handle_mapped_error(err),
    }
}

/// Exclude a tenant owner
///
/// A single tenant can have multiple owners. This endpoint allows to exclude an
/// owner from the tenant.
///
#[utoipa::path(
    delete,
    params(
        ("id" = Uuid, Path, description = "The tenant primary key."),
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
            status = 204,
            description = "Owner deleted.",
        ),
    ),
)]
#[delete("/{id}/owner/{owner_id}")]
pub async fn exclude_tenant_owner_url(
    path: web::Path<(Uuid, Uuid)>,
    profile: MyceliumProfileData,
    app_module: web::Data<SqlAppModule>,
) -> impl Responder {
    let (tenant_id, owner_id) = path.into_inner();

    match exclude_tenant_owner(
        profile.to_profile(),
        tenant_id,
        owner_id,
        Box::new(&*app_module.resolve_ref()),
    )
    .await
    {
        Ok(res) => delete_response_kind(res),
        Err(err) => handle_mapped_error(err),
    }
}
