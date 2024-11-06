use crate::{
    dtos::MyceliumProfileData,
    endpoints::{shared::UrlGroup, standard::shared::build_actor_context},
    modules::{MessageSendingQueueModule, TokenRegistrationModule},
};

use actix_web::{post, web, HttpResponse, Responder};
use myc_core::{
    domain::{
        actors::ActorName,
        dtos::route_type::PermissionedRoles,
        entities::{MessageSending, TokenRegistration},
    },
    models::AccountLifeCycle,
    use_cases::roles::standard::guest_manager::token::create_default_account_associated_connection_string,
};
use myc_http_tools::wrappers::default_response_to_http_response::handle_mapped_error;
use serde::{Deserialize, Serialize};
use shaku_actix::Inject;
use utoipa::ToSchema;
use uuid::Uuid;

// ? ---------------------------------------------------------------------------
// ? Configure application
// ? ---------------------------------------------------------------------------

pub fn configure(config: &mut web::ServiceConfig) {
    config.service(create_default_account_associated_connection_string_url);
}

// ? ---------------------------------------------------------------------------
// ? Define API structs
// ? ---------------------------------------------------------------------------

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateDefaultAccountAssociatedTokenBody {
    tenant_id: Uuid,
    account_id: Uuid,
    permissioned_roles: PermissionedRoles,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateDefaultAccountAssociatedTokenResponse {
    connection_string: String,
}

/// Create Guest Role
///
/// Guest Roles provide permissions to simple Roles.
#[utoipa::path(
    post,
    context_path = build_actor_context(ActorName::GuestManager, UrlGroup::Tokens),
    request_body = CreateDefaultAccountAssociatedTokenBody,
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
            description = "Token created.",
            body = CreateDefaultAccountAssociatedTokenResponse,
        ),
    ),
)]
#[post("/")]
pub async fn create_default_account_associated_connection_string_url(
    json: web::Json<CreateDefaultAccountAssociatedTokenBody>,
    profile: MyceliumProfileData,
    life_cycle_settings: web::Data<AccountLifeCycle>,
    token_registration_repo: Inject<
        TokenRegistrationModule,
        dyn TokenRegistration,
    >,
    message_sending_repo: Inject<MessageSendingQueueModule, dyn MessageSending>,
) -> impl Responder {
    match create_default_account_associated_connection_string(
        profile.to_profile(),
        json.tenant_id.to_owned(),
        json.account_id.to_owned(),
        json.permissioned_roles.to_owned(),
        life_cycle_settings.get_ref().to_owned(),
        Box::new(&*token_registration_repo),
        Box::new(&*message_sending_repo),
    )
    .await
    {
        Ok(res) => HttpResponse::Ok().json(
            CreateDefaultAccountAssociatedTokenResponse {
                connection_string: res,
            },
        ),
        Err(err) => handle_mapped_error(err),
    }
}
