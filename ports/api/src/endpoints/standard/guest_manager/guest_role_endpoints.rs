use crate::{
    dtos::MyceliumProfileData,
    endpoints::{shared::UrlGroup, standard::shared::build_actor_context},
    modules::{
        GuestRoleDeletionModule, GuestRoleFetchingModule,
        GuestRoleRegistrationModule, GuestRoleUpdatingModule,
    },
};

use actix_web::{delete, get, patch, post, web, Responder};
use myc_core::{
    domain::{
        actors::ActorName,
        dtos::guest_role::Permission,
        entities::{
            GuestRoleDeletion, GuestRoleFetching, GuestRoleRegistration,
            GuestRoleUpdating,
        },
    },
    use_cases::roles::standard::guest_manager::guest_role::{
        create_guest_role, delete_guest_role, insert_role_child,
        list_guest_roles, remove_role_child,
        update_guest_role_name_and_description, update_guest_role_permission,
    },
};
use myc_http_tools::wrappers::default_response_to_http_response::{
    delete_response_kind, fetch_many_response_kind,
    get_or_create_response_kind, handle_mapped_error, updating_response_kind,
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
    pub role_id: Uuid,
}

#[derive(Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct ListGuestRolesParams {
    pub name: Option<String>,
    pub role_id: Option<Uuid>,
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
    context_path = build_actor_context(ActorName::GuestManager, UrlGroup::GuestRoles),
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
#[post("/")]
pub async fn crate_guest_role_url(
    json: web::Json<CreateGuestRoleBody>,
    profile: MyceliumProfileData,
    guest_role_registration_repo: Inject<
        GuestRoleRegistrationModule,
        dyn GuestRoleRegistration,
    >,
) -> impl Responder {
    match create_guest_role(
        profile.to_profile(),
        json.name.to_owned(),
        json.description.to_owned(),
        json.role_id.to_owned(),
        None,
        Box::new(&*guest_role_registration_repo),
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
    context_path = build_actor_context(ActorName::GuestManager, UrlGroup::GuestRoles),
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
            body = [Role],
        ),
    ),
)]
#[get("/")]
pub async fn list_guest_roles_url(
    info: web::Query<ListGuestRolesParams>,
    profile: MyceliumProfileData,
    guest_role_fetching_repo: Inject<
        GuestRoleFetchingModule,
        dyn GuestRoleFetching,
    >,
) -> impl Responder {
    match list_guest_roles(
        profile.to_profile(),
        info.name.to_owned(),
        info.role_id.to_owned(),
        Box::new(&*guest_role_fetching_repo),
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
    context_path = build_actor_context(ActorName::GuestManager, UrlGroup::GuestRoles),
    params(
        ("role_id" = Uuid, Path, description = "The guest-role primary key."),
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
#[delete("/{role_id}")]
pub async fn delete_guest_role_url(
    path: web::Path<Uuid>,
    profile: MyceliumProfileData,
    role_deletion_repo: Inject<GuestRoleDeletionModule, dyn GuestRoleDeletion>,
) -> impl Responder {
    match delete_guest_role(
        profile.to_profile(),
        path.to_owned(),
        Box::new(&*role_deletion_repo),
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
    context_path = build_actor_context(ActorName::GuestManager, UrlGroup::GuestRoles),
    params(
        ("role_id" = Uuid, Path, description = "The guest-role primary key."),
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
#[patch("/{role_id}")]
pub async fn update_guest_role_name_and_description_url(
    path: web::Path<Uuid>,
    body: web::Json<UpdateGuestRoleNameAndDescriptionBody>,
    profile: MyceliumProfileData,
    role_fetching_repo: Inject<GuestRoleFetchingModule, dyn GuestRoleFetching>,
    role_updating_repo: Inject<GuestRoleUpdatingModule, dyn GuestRoleUpdating>,
) -> impl Responder {
    match update_guest_role_name_and_description(
        profile.to_profile(),
        body.name.to_owned(),
        body.description.to_owned(),
        path.to_owned(),
        Box::new(&*role_fetching_repo),
        Box::new(&*role_updating_repo),
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
    context_path = build_actor_context(ActorName::GuestManager, UrlGroup::GuestRoles),
    params(
        ("role" = Uuid, Path, description = "The guest-role primary key."),
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
#[patch("/{role_id}/permissions")]
pub async fn update_guest_role_permissions_url(
    path: web::Path<Uuid>,
    body: web::Json<UpdateGuestRolePermissionsBody>,
    profile: MyceliumProfileData,
    role_fetching_repo: Inject<GuestRoleFetchingModule, dyn GuestRoleFetching>,
    role_updating_repo: Inject<GuestRoleUpdatingModule, dyn GuestRoleUpdating>,
) -> impl Responder {
    match update_guest_role_permission(
        profile.to_profile(),
        path.to_owned(),
        body.permission.to_owned(),
        Box::new(&*role_fetching_repo),
        Box::new(&*role_updating_repo),
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
    context_path = build_actor_context(ActorName::GuestManager, UrlGroup::GuestRoles),
    params(
        ("role_id" = Uuid, Path, description = "The guest-role primary key."),
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
#[post("/{role_id}/children/{child_id}")]
pub async fn insert_role_child_url(
    path: web::Path<(Uuid, Uuid)>,
    profile: MyceliumProfileData,
    guest_role_fetching_repo: Inject<
        GuestRoleFetchingModule,
        dyn GuestRoleFetching,
    >,
    guest_role_updating_repo: Inject<
        GuestRoleUpdatingModule,
        dyn GuestRoleUpdating,
    >,
) -> impl Responder {
    let (role_id, child_id) = path.into_inner();

    match insert_role_child(
        profile.to_profile(),
        role_id,
        child_id,
        Box::new(&*guest_role_fetching_repo),
        Box::new(&*guest_role_updating_repo),
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
    context_path = build_actor_context(ActorName::GuestManager, UrlGroup::GuestRoles),
    params(
        ("role_id" = Uuid, Path, description = "The guest-role primary key."),
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
#[delete("/{role_id}/children/{child_id}")]
pub async fn remove_role_child_url(
    path: web::Path<(Uuid, Uuid)>,
    profile: MyceliumProfileData,
    guest_role_updating_repo: Inject<
        GuestRoleUpdatingModule,
        dyn GuestRoleUpdating,
    >,
) -> impl Responder {
    let (role_id, child_id) = path.into_inner();

    match remove_role_child(
        profile.to_profile(),
        role_id,
        child_id,
        Box::new(&*guest_role_updating_repo),
    )
    .await
    {
        Ok(res) => updating_response_kind(res),
        Err(err) => handle_mapped_error(err),
    }
}
