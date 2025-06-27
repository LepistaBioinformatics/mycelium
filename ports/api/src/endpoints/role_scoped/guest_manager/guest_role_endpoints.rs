use crate::{dtos::MyceliumProfileData, endpoints::shared::PaginationParams};

use actix_web::{delete, get, patch, post, web, Responder};
use myc_core::{
    domain::dtos::guest_role::{GuestRole, Permission},
    use_cases::role_scoped::guest_manager::guest_role::{
        create_guest_role, delete_guest_role, insert_role_child,
        list_guest_roles, remove_role_child,
        update_guest_role_name_and_description, update_guest_role_permission,
    },
};
use myc_diesel::repositories::SqlAppModule;
use myc_http_tools::{
    utils::HttpJsonResponse,
    wrappers::default_response_to_http_response::{
        delete_response_kind, fetch_many_response_kind,
        get_or_create_response_kind, handle_mapped_error,
        updating_response_kind,
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
        .service(crate_guest_role_url)
        .service(list_guest_roles_url)
        .service(delete_guest_role_url)
        .service(update_guest_role_name_and_description_url)
        .service(update_guest_role_permissions_url)
        .service(insert_role_child_url)
        .service(remove_role_child_url);
}

// ? ---------------------------------------------------------------------------
// ? Define API structs
// ? ---------------------------------------------------------------------------

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateGuestRoleBody {
    pub name: String,
    pub description: String,
    pub permission: Option<Permission>,
    pub system: bool,
}

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

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateGuestRoleNameAndDescriptionBody {
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateGuestRolePermissionsBody {
    pub permission: Permission,
}

/// Create Guest Role
///
/// Guest Roles provide permissions to simple Roles.
#[utoipa::path(
    post,
    request_body = CreateGuestRoleBody,
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
            description = "Guest Role created.",
            body = GuestRole,
        ),
        (
            status = 200,
            description = "Guest Role already exists.",
            body = GuestRole,
        ),
    ),
)]
#[post("")]
pub async fn crate_guest_role_url(
    json: web::Json<CreateGuestRoleBody>,
    profile: MyceliumProfileData,
    app_module: web::Data<SqlAppModule>,
) -> impl Responder {
    match create_guest_role(
        profile.to_profile(),
        json.name.to_owned(),
        json.description.to_owned(),
        json.permission.to_owned(),
        json.system,
        Box::new(&*app_module.resolve_ref()),
    )
    .await
    {
        Ok(res) => get_or_create_response_kind(res),
        Err(err) => handle_mapped_error(err),
    }
}

/// List Roles
#[utoipa::path(
    get,
    params(
        ListGuestRolesParams,
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
    info: web::Query<ListGuestRolesParams>,
    page: web::Query<PaginationParams>,
    profile: MyceliumProfileData,
    app_module: web::Data<SqlAppModule>,
) -> impl Responder {
    match list_guest_roles(
        profile.to_profile(),
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

/// Delete Guest Role
///
/// Delete a single guest role.
#[utoipa::path(
    delete,
    params(
        ("guest_role_id" = Uuid, Path, description = "The guest-role primary key."),
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
            description = "Guest Role not deleted.",
            body = HttpJsonResponse,
        ),
        (
            status = 204,
            description = "Guest Role deleted.",
        ),
    ),
)]
#[delete("/{guest_role_id}")]
pub async fn delete_guest_role_url(
    path: web::Path<Uuid>,
    profile: MyceliumProfileData,
    app_module: web::Data<SqlAppModule>,
) -> impl Responder {
    match delete_guest_role(
        profile.to_profile(),
        path.to_owned(),
        Box::new(&*app_module.resolve_ref()),
    )
    .await
    {
        Ok(res) => delete_response_kind(res),
        Err(err) => handle_mapped_error(err),
    }
}

/// Partial Update Guest Role
///
/// Update name and description of a single Guest Role.
#[utoipa::path(
    patch,
    params(
        ("guest_role_id" = Uuid, Path, description = "The guest-role primary key."),
    ),
    request_body = UpdateGuestRoleNameAndDescriptionBody,
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
            description = "Guest Role not deleted.",
            body = HttpJsonResponse,
        ),
        (
            status = 202,
            description = "Guest Role updated.",
            body = GuestRole,
        ),
    ),
)]
#[patch("/{guest_role_id}")]
pub async fn update_guest_role_name_and_description_url(
    path: web::Path<Uuid>,
    body: web::Json<UpdateGuestRoleNameAndDescriptionBody>,
    profile: MyceliumProfileData,
    app_module: web::Data<SqlAppModule>,
) -> impl Responder {
    match update_guest_role_name_and_description(
        profile.to_profile(),
        body.name.to_owned(),
        body.description.to_owned(),
        path.to_owned(),
        Box::new(&*app_module.resolve_ref()),
        Box::new(&*app_module.resolve_ref()),
    )
    .await
    {
        Ok(res) => updating_response_kind(res),
        Err(err) => handle_mapped_error(err),
    }
}

/// Change permissions of Guest Role
///
/// Upgrade or Downgrade permissions of Guest Role.
#[utoipa::path(
    patch,
    params(
        ("guest_role_id" = Uuid, Path, description = "The guest-role primary key."),
    ),
    request_body = UpdateGuestRolePermissionsBody,
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
            description = "Guest Role not deleted.",
            body = HttpJsonResponse,
        ),
        (
            status = 202,
            description = "Guest Role updated.",
            body = GuestRole,
        ),
    ),
)]
#[patch("/{guest_role_id}/permissions")]
pub async fn update_guest_role_permissions_url(
    path: web::Path<Uuid>,
    body: web::Json<UpdateGuestRolePermissionsBody>,
    profile: MyceliumProfileData,
    app_module: web::Data<SqlAppModule>,
) -> impl Responder {
    match update_guest_role_permission(
        profile.to_profile(),
        path.to_owned(),
        body.permission.to_owned(),
        Box::new(&*app_module.resolve_ref()),
        Box::new(&*app_module.resolve_ref()),
    )
    .await
    {
        Ok(res) => updating_response_kind(res),
        Err(err) => handle_mapped_error(err),
    }
}

/// Set Child Role
///
/// Insert a child role to a parent role.
#[utoipa::path(
    post,
    params(
        ("guest_role_id" = Uuid, Path, description = "The guest-role primary key."),
        ("child_id" = Uuid, Path, description = "The child guest-role primary key."),
    ),
    request_body = UpdateGuestRolePermissionsBody,
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
            description = "Guest Role not deleted.",
            body = HttpJsonResponse,
        ),
        (
            status = 202,
            description = "Guest Role updated.",
            body = GuestRole,
        ),
    ),
)]
#[post("/{guest_role_id}/children/{child_id}")]
pub async fn insert_role_child_url(
    path: web::Path<(Uuid, Uuid)>,
    profile: MyceliumProfileData,
    app_module: web::Data<SqlAppModule>,
) -> impl Responder {
    let (guest_role_id, child_id) = path.into_inner();

    match insert_role_child(
        profile.to_profile(),
        guest_role_id,
        child_id,
        Box::new(&*app_module.resolve_ref()),
        Box::new(&*app_module.resolve_ref()),
    )
    .await
    {
        Ok(res) => updating_response_kind(res),
        Err(err) => handle_mapped_error(err),
    }
}

/// Delete Child Role
///
/// Delete a child role to a parent role.
#[utoipa::path(
    delete,
    params(
        ("guest_role_id" = Uuid, Path, description = "The guest-role primary key."),
        ("child_id" = Uuid, Path, description = "The child guest-role primary key."),
    ),
    request_body = UpdateGuestRolePermissionsBody,
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
            description = "Guest Role not deleted.",
            body = HttpJsonResponse,
        ),
        (
            status = 202,
            description = "Guest Role updated.",
            body = GuestRole,
        ),
    ),
)]
#[delete("/{guest_role_id}/children/{child_id}")]
pub async fn remove_role_child_url(
    path: web::Path<(Uuid, Uuid)>,
    profile: MyceliumProfileData,
    app_module: web::Data<SqlAppModule>,
) -> impl Responder {
    let (guest_role_id, child_id) = path.into_inner();

    match remove_role_child(
        profile.to_profile(),
        guest_role_id,
        child_id,
        Box::new(&*app_module.resolve_ref()),
    )
    .await
    {
        Ok(res) => updating_response_kind(res),
        Err(err) => handle_mapped_error(err),
    }
}
