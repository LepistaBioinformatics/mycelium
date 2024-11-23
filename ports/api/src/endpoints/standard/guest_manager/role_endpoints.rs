use crate::{
    dtos::MyceliumProfileData,
    endpoints::shared::{build_actor_context, UrlGroup},
    modules::{
        RoleDeletionModule, RoleFetchingModule, RoleRegistrationModule,
        RoleUpdatingModule,
    },
};

use actix_web::{delete, get, patch, post, web, Responder};
use myc_core::{
    domain::{
        actors::ActorName,
        entities::{
            RoleDeletion, RoleFetching, RoleRegistration, RoleUpdating,
        },
    },
    use_cases::roles::standard::guest_manager::role::{
        create_role, delete_role, list_roles, update_role_name_and_description,
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
        .service(crate_role_url)
        .service(list_roles_url)
        .service(delete_role_url)
        .service(update_role_name_and_description_url);
}

// ? ---------------------------------------------------------------------------
// ? Define API structs
// ? ---------------------------------------------------------------------------

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateRoleBody {
    pub name: String,
    pub description: String,
}

#[derive(Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct ListRolesParams {
    pub name: Option<String>,
}

/// Create Role
///
/// Roles are used to build Guest Role elements.
#[utoipa::path(
    post,
    context_path = build_actor_context(ActorName::GuestManager, UrlGroup::Roles),
    request_body = CreateRoleBody,
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
            description = "Role created.",
            body = Role,
        ),
        (
            status = 200,
            description = "Role already exists.",
            body = Role,
        ),
    ),
)]
#[post("/")]
pub async fn crate_role_url(
    body: web::Json<CreateRoleBody>,
    profile: MyceliumProfileData,
    role_registration_repo: Inject<
        RoleRegistrationModule,
        dyn RoleRegistration,
    >,
) -> impl Responder {
    match create_role(
        profile.to_profile(),
        body.name.to_owned(),
        body.description.to_owned(),
        Box::new(&*role_registration_repo),
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
    context_path = build_actor_context(ActorName::GuestManager, UrlGroup::Roles),
    params(
        ListRolesParams,
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
            status = 401,
            description = "Unauthorized.",
            body = HttpJsonResponse,
        ),
        (
            status = 403,
            description = "Forbidden.",
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
pub async fn list_roles_url(
    info: web::Query<ListRolesParams>,
    profile: MyceliumProfileData,
    roles_fetching_repo: Inject<RoleFetchingModule, dyn RoleFetching>,
) -> impl Responder {
    let name = info.name.to_owned();

    match list_roles(
        profile.to_profile(),
        name.to_owned(),
        Box::new(&*roles_fetching_repo),
    )
    .await
    {
        Ok(res) => fetch_many_response_kind(res),
        Err(err) => handle_mapped_error(err),
    }
}

/// Delete Role
///
/// Delete a single role.
#[utoipa::path(
    delete,
    context_path = build_actor_context(ActorName::GuestManager, UrlGroup::Roles),
    params(
        ("role_id" = Uuid, Path, description = "The role primary key."),
    ),
    responses(
        (
            status = 500,
            description = "Unknown internal server error.",
            body = HttpJsonResponse,
        ),
        (
            status = 400,
            description = "Role not deleted.",
            body = HttpJsonResponse,
        ),
        (
            status = 401,
            description = "Unauthorized.",
            body = HttpJsonResponse,
        ),
        (
            status = 403,
            description = "Forbidden.",
            body = HttpJsonResponse,
        ),
        (
            status = 204,
            description = "Role deleted.",
        ),
    ),
)]
#[delete("/{role_id}")]
pub async fn delete_role_url(
    path: web::Path<Uuid>,
    profile: MyceliumProfileData,
    role_deletion_repo: Inject<RoleDeletionModule, dyn RoleDeletion>,
) -> impl Responder {
    match delete_role(
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

/// Partial Update Role
///
/// Update name and description of a single Role.
#[utoipa::path(
    patch,
    context_path = build_actor_context(ActorName::GuestManager, UrlGroup::Roles),
    params(
        ("role_id" = Uuid, Path, description = "The role primary key."),
    ),
    request_body = CreateRoleBody,
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
            body = Role,
        ),
    ),
)]
#[patch("/{role_id}")]
pub async fn update_role_name_and_description_url(
    path: web::Path<Uuid>,
    body: web::Json<CreateRoleBody>,
    profile: MyceliumProfileData,
    role_fetching_repo: Inject<RoleFetchingModule, dyn RoleFetching>,
    role_updating_repo: Inject<RoleUpdatingModule, dyn RoleUpdating>,
) -> impl Responder {
    match update_role_name_and_description(
        profile.to_profile(),
        path.to_owned(),
        body.name.to_owned(),
        body.description.to_owned(),
        Box::new(&*role_fetching_repo),
        Box::new(&*role_updating_repo),
    )
    .await
    {
        Ok(res) => updating_response_kind(res),
        Err(err) => handle_mapped_error(err),
    }
}
