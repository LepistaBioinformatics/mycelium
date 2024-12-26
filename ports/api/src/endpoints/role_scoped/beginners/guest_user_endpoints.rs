use crate::{
    dtos::MyceliumProfileData, modules::GuestUserOnAccountUpdatingModule,
};

use actix_web::{post, web, Responder};
use myc_core::{
    domain::entities::GuestUserOnAccountUpdating,
    use_cases::roles::role_scoped::beginner::guest_user::accept_invitation,
};
use myc_http_tools::{
    utils::HttpJsonResponse,
    wrappers::default_response_to_http_response::{
        handle_mapped_error, updating_response_kind,
    },
    Permission, Profile,
};
use shaku_actix::Inject;
use uuid::Uuid;

// ? ---------------------------------------------------------------------------
// ? Configure application
// ? ---------------------------------------------------------------------------

pub fn configure(config: &mut web::ServiceConfig) {
    config.service(accept_invitation_url);
}

// ? ---------------------------------------------------------------------------
// ? Define API paths
//
// Account
//
// ? ---------------------------------------------------------------------------

/// Fetch a user's profile.
#[utoipa::path(
    post,
    params(
        ("account_id" = Uuid, Path, description = "The account primary key."),
        ("guest_role_name" = Uuid, Path, description = "The guest role unique name."),
        ("permission" = u8, Path, description = "The permission to be granted."),
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
            description = "Invitation accepted",
            body = Profile,
        ),
    ),
)]
#[post(
    "/account/{account_id}/guest-role/{guest_role_name}/perm/{permission}/accept"
)]
pub async fn accept_invitation_url(
    query: web::Query<(Uuid, String, u8)>,
    profile: MyceliumProfileData,
    guest_user_on_account_repo: Inject<
        GuestUserOnAccountUpdatingModule,
        dyn GuestUserOnAccountUpdating,
    >,
) -> impl Responder {
    let (account_id, guest_role_name, permission) = query.into_inner();

    match accept_invitation(
        profile.to_profile(),
        account_id,
        guest_role_name,
        Permission::from_i32(permission.into()),
        Box::new(&*guest_user_on_account_repo),
    )
    .await
    {
        Ok(response) => updating_response_kind(response),
        Err(err) => handle_mapped_error(err),
    }
}
