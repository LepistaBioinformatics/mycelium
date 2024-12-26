use crate::{
    dtos::MyceliumProfileData,
    modules::{GuestRoleRegistrationModule, RoleRegistrationModule},
};

use actix_web::{post, web, HttpResponse, Responder};
use myc_core::{
    domain::{
        dtos::guest_role::GuestRole,
        entities::{GuestRoleRegistration, RoleRegistration},
    },
    use_cases::roles::super_users::managers::create_system_roles,
};
use myc_http_tools::{
    utils::HttpJsonResponse,
    wrappers::default_response_to_http_response::handle_mapped_error,
};
use shaku_actix::Inject;

// ? ---------------------------------------------------------------------------
// ? Configure application
// ? ---------------------------------------------------------------------------

pub fn configure(config: &mut web::ServiceConfig) {
    config.service(web::scope("/guest-roles").service(create_system_roles_url));
}

// ? ---------------------------------------------------------------------------
// ? Define API structs
// ? ---------------------------------------------------------------------------

// TODO

// ? ---------------------------------------------------------------------------
// ? Define API paths
// ? ---------------------------------------------------------------------------

/// Create all system roles
#[utoipa::path(
    post,
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
            description = "Guest roles created.",
            body = [GuestRole],
        ),
    ),
)]
#[post("")]
pub async fn create_system_roles_url(
    profile: MyceliumProfileData,
    role_registration_repo: Inject<
        RoleRegistrationModule,
        dyn RoleRegistration,
    >,
    guest_role_registration_repo: Inject<
        GuestRoleRegistrationModule,
        dyn GuestRoleRegistration,
    >,
) -> impl Responder {
    match create_system_roles(
        profile.to_profile(),
        Box::new(&*role_registration_repo),
        Box::new(&*guest_role_registration_repo),
    )
    .await
    {
        Ok(res) => HttpResponse::Created().json(res),
        Err(err) => handle_mapped_error(err),
    }
}
