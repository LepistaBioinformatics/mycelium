use crate::{
    dtos::MyceliumProfileData,
    endpoints::shared::{build_actor_context, UrlGroup},
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
    use_cases::roles::standard::guest_manager::token::{
        create_default_account_associated_connection_string,
        create_role_associated_connection_string,
    },
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
    config
        .service(create_default_account_associated_connection_string_url)
        .service(create_role_associated_connection_string_url);
}

// ? ---------------------------------------------------------------------------
// ? Define API structs
// ? ---------------------------------------------------------------------------

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateTokenBody {
    tenant_id: Uuid,
    permissioned_roles: PermissionedRoles,
    expiration: i64,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateTokenResponse {
    connection_string: String,
}

/// Create Account Associated Token
///
/// This action creates a token that is associated with the account specified
/// in the `account_id` argument. The token is scoped to the roles specified
/// in the `permissioned_roles` argument.
///
#[utoipa::path(
    post,
    context_path = build_actor_context(ActorName::GuestManager, UrlGroup::Tokens),
    params(
        ("account_id" = Uuid, Path, description = "The account unique id."),
    ),
    request_body = CreateTokenBody,
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
#[post("/accounts/{account_id}")]
pub async fn create_default_account_associated_connection_string_url(
    path: web::Path<Uuid>,
    body: web::Json<CreateTokenBody>,
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
        body.tenant_id.to_owned(),
        path.to_owned(),
        body.expiration.to_owned(),
        body.permissioned_roles.to_owned(),
        life_cycle_settings.get_ref().to_owned(),
        Box::new(&*token_registration_repo),
        Box::new(&*message_sending_repo),
    )
    .await
    {
        Ok(connection_string) => {
            HttpResponse::Ok().json(CreateTokenResponse { connection_string })
        }
        Err(err) => handle_mapped_error(err),
    }
}

/// Create Role Associated Token
///
/// This action creates a token that is associated with the role specified
/// in the `role_id` argument. The token is scoped to the roles specified
/// in the `permissioned_roles` argument.
///
#[utoipa::path(
    post,
    context_path = build_actor_context(ActorName::GuestManager, UrlGroup::Tokens),
    params(
        ("role_id" = Uuid, Path, description = "The role unique id."),
    ),
    request_body = CreateTokenBody,
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
#[post("/roles/{role_id}")]
pub async fn create_role_associated_connection_string_url(
    path: web::Path<Uuid>,
    body: web::Json<CreateTokenBody>,
    profile: MyceliumProfileData,
    life_cycle_settings: web::Data<AccountLifeCycle>,
    token_registration_repo: Inject<
        TokenRegistrationModule,
        dyn TokenRegistration,
    >,
    message_sending_repo: Inject<MessageSendingQueueModule, dyn MessageSending>,
) -> impl Responder {
    match create_role_associated_connection_string(
        profile.to_profile(),
        body.tenant_id.to_owned(),
        path.to_owned(),
        body.expiration.to_owned(),
        body.permissioned_roles.to_owned(),
        life_cycle_settings.get_ref().to_owned(),
        Box::new(&*token_registration_repo),
        Box::new(&*message_sending_repo),
    )
    .await
    {
        Ok(connection_string) => {
            HttpResponse::Ok().json(CreateTokenResponse { connection_string })
        }
        Err(err) => handle_mapped_error(err),
    }
}
