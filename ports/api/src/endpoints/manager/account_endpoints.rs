use crate::{
    dtos::MyceliumProfileData,
    modules::{AccountRegistrationModule, GuestRoleFetchingModule},
};

use actix_web::{post, web, Responder};
use myc_core::{
    domain::{
        dtos::guest_role::GuestRole,
        entities::{AccountRegistration, GuestRoleFetching},
    },
    use_cases::super_users::managers::create_system_account,
};
use myc_http_tools::{
    utils::HttpJsonResponse,
    wrappers::default_response_to_http_response::{
        get_or_create_response_kind, handle_mapped_error,
    },
    SystemActor,
};
use serde::Deserialize;
use shaku_actix::Inject;
use utoipa::ToSchema;
use uuid::Uuid;

// ? ---------------------------------------------------------------------------
// ? Configure application
// ? ---------------------------------------------------------------------------

pub fn configure(config: &mut web::ServiceConfig) {
    config.service(web::scope("/accounts").service(create_system_account_url));
}

// ? ---------------------------------------------------------------------------
// ? Define API structs
// ? ---------------------------------------------------------------------------

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateSystemSubscriptionAccountBody {
    /// The account name
    name: String,

    /// The tenant ID
    tenant_id: Uuid,

    /// The role name
    role: SystemActor,

    /// The role ID
    role_id: Uuid,
}

// ? ---------------------------------------------------------------------------
// ? Define API paths
// ? ---------------------------------------------------------------------------

/// Create all system roles
#[utoipa::path(
    post,
    request_body = CreateSystemSubscriptionAccountBody,
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
            description = "Account created.",
            body = [GuestRole],
        ),
    ),
)]
#[post("")]
pub async fn create_system_account_url(
    body: web::Json<CreateSystemSubscriptionAccountBody>,
    profile: MyceliumProfileData,
    guest_role_fetching_repo: Inject<
        GuestRoleFetchingModule,
        dyn GuestRoleFetching,
    >,
    account_registration_repo: Inject<
        AccountRegistrationModule,
        dyn AccountRegistration,
    >,
) -> impl Responder {
    match create_system_account(
        profile.to_profile(),
        body.name.to_owned(),
        body.tenant_id.to_owned(),
        body.role.to_owned(),
        body.role_id.to_owned(),
        Box::new(&*guest_role_fetching_repo),
        Box::new(&*account_registration_repo),
    )
    .await
    {
        Ok(res) => get_or_create_response_kind(res),
        Err(err) => handle_mapped_error(err),
    }
}
