use crate::dtos::MyceliumProfileData;

use actix_web::{post, web, Responder};
use myc_core::{
    domain::dtos::guest_role::GuestRole,
    use_cases::super_users::managers::create_system_account,
};
use myc_diesel::repositories::SqlAppModule;
use myc_http_tools::{
    utils::HttpJsonResponse,
    wrappers::default_response_to_http_response::{
        get_or_create_response_kind, handle_mapped_error,
    },
    SystemActor,
};
use serde::Deserialize;
use shaku::HasComponent;
use utoipa::ToSchema;

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
pub(crate) enum ApiSystemActor {
    GatewayManager,
    GuestsManager,
    SystemManager,
}

impl ApiSystemActor {
    fn to_system_actor(&self) -> SystemActor {
        match self {
            ApiSystemActor::GatewayManager => SystemActor::GatewayManager,
            ApiSystemActor::GuestsManager => SystemActor::GuestsManager,
            ApiSystemActor::SystemManager => SystemActor::SystemManager,
        }
    }
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateSystemSubscriptionAccountBody {
    /// The account name
    name: String,

    /// The role ID
    actor: ApiSystemActor,
}

// ? ---------------------------------------------------------------------------
// ? Define API paths
// ? ---------------------------------------------------------------------------

/// Create system account
#[utoipa::path(
    post,
    operation_id = "create_system_level_account",
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
    app_module: web::Data<SqlAppModule>,
) -> impl Responder {
    match create_system_account(
        profile.to_profile(),
        body.name.to_owned(),
        body.actor.to_system_actor(),
        Box::new(&*app_module.resolve_ref()),
    )
    .await
    {
        Ok(res) => get_or_create_response_kind(res),
        Err(err) => handle_mapped_error(err),
    }
}
