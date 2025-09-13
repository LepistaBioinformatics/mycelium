use crate::{
    dtos::{MyceliumProfileData, TenantData},
    endpoints::shared::PaginationParams,
};

use actix_web::{get, web, Responder};
use myc_core::{
    domain::dtos::guest_role::GuestRole,
    use_cases::role_scoped::subscriptions_manager::guest_role::{
        fetch_guest_role_details, list_guest_roles,
    },
};
use myc_diesel::repositories::SqlAppModule;
use myc_http_tools::{
    utils::HttpJsonResponse,
    wrappers::default_response_to_http_response::{
        fetch_many_response_kind, fetch_response_kind, handle_mapped_error,
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
    config
        .service(list_guest_roles_url)
        .service(fetch_guest_role_details_url);
}

// ? ---------------------------------------------------------------------------
// ? Define API structs
// ? ---------------------------------------------------------------------------

#[derive(Deserialize, ToSchema, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct ListGuestRolesParams {
    /// The name of the guest role.
    pub name: Option<String>,

    /// The slug of the guest role.
    pub slug: Option<String>,

    /// If it is a system role.
    pub system: Option<bool>,
}

/// List Roles
#[utoipa::path(
    get,
    operation_id = "list_guest_roles",
    params(
        ListGuestRolesParams,
        (
            "x-mycelium-tenant-id" = Uuid,
            Header,
            description = "The tenant unique id."
        )
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
            description = "Success.",
            body = [GuestRole],
        ),
    ),
)]
#[get("")]
pub async fn list_guest_roles_url(
    tenant: Option<TenantData>,
    info: web::Query<ListGuestRolesParams>,
    page: web::Query<PaginationParams>,
    profile: MyceliumProfileData,
    app_module: web::Data<SqlAppModule>,
) -> impl Responder {
    let tenant_id = match tenant {
        Some(tenant) => Some(tenant.tenant_id().to_owned()),
        None => None,
    };

    match list_guest_roles(
        profile.to_profile(),
        tenant_id.to_owned(),
        info.name.to_owned(),
        info.slug.to_owned(),
        info.system.to_owned(),
        page.page_size,
        page.skip,
        Box::new(&*app_module.resolve_ref()),
    )
    .await
    {
        Ok(res) => fetch_many_response_kind(res),
        Err(err) => handle_mapped_error(err),
    }
}

/// Fetch Guest Role Details
#[utoipa::path(
    get,
    operation_id = "fetch_guest_role_details",
    params(
        ("id" = Uuid, Path, description = "The guest role primary key."),
        (
            "x-mycelium-tenant-id" = Uuid,
            Header,
            description = "The tenant unique id."
        )
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
            description = "Success.",
            body = GuestRole,
        ),
    ),
)]
#[get("/{id}")]
pub async fn fetch_guest_role_details_url(
    tenant: Option<TenantData>,
    path: web::Path<Uuid>,
    profile: MyceliumProfileData,
    app_module: web::Data<SqlAppModule>,
) -> impl Responder {
    let tenant_id = match tenant {
        Some(tenant) => Some(tenant.tenant_id().to_owned()),
        None => None,
    };

    match fetch_guest_role_details(
        profile.to_profile(),
        tenant_id.to_owned(),
        path.to_owned(),
        Box::new(&*app_module.resolve_ref()),
    )
    .await
    {
        Ok(res) => fetch_response_kind(res),
        Err(err) => handle_mapped_error(err),
    }
}
