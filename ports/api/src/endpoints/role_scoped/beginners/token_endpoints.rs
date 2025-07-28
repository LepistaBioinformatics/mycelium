use crate::dtos::MyceliumProfileData;

use actix_web::{post, web, HttpResponse, Responder};
use myc_core::{
    domain::dtos::security_group::PermissionedRole, models::AccountLifeCycle,
    use_cases::role_scoped::beginner::token::create_connection_string,
};
use myc_diesel::repositories::SqlAppModule;
use myc_http_tools::{
    utils::HttpJsonResponse,
    wrappers::default_response_to_http_response::handle_mapped_error,
};
use serde::{Deserialize, Serialize};
use shaku::HasComponent;
use utoipa::{ToResponse, ToSchema};
use uuid::Uuid;

// ? ---------------------------------------------------------------------------
// ? Configure application
// ? ---------------------------------------------------------------------------

pub fn configure(config: &mut web::ServiceConfig) {
    config.service(create_connection_string_url);
}

// ? ---------------------------------------------------------------------------
// ? Define API structs
// ? ---------------------------------------------------------------------------

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateTokenBody {
    /// The expiration time of the token
    ///
    /// The expiration time of the token in seconds.
    ///
    expiration: i64,

    /// A single tenant ID
    ///
    /// If specified, the actions allowed by the token will be scoped to the
    /// tenant. If not specified, the actions allowed by the token will be
    /// scoped to the user profile.
    ///
    tenant_id: Option<Uuid>,

    /// A single service account ID
    ///
    /// If specified, the actions allowed by the token will be scoped to the
    /// service account. Service account should be a subscription account,
    /// tenant management account, or role scoped account. If not specified, the
    /// actions allowed by the token will be scoped to the user profile.
    ///
    service_account_id: Option<Uuid>,

    /// A list of roles
    ///
    /// If specified, the actions allowed by the token will be scoped to the
    /// roles. If not specified, the actions allowed by the token will be
    /// scoped to the user profile.
    ///
    roles: Option<Vec<PermissionedRole>>,
}

#[derive(Serialize, ToSchema, ToResponse)]
#[serde(rename_all = "camelCase")]
pub struct CreateTokenResponse {
    connection_string: String,
}

/// Create Connection String
///
/// This action creates a connection string that is associated with the user
/// account. The connection string has the same permissions of the user account.
///
#[utoipa::path(
    post,
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
            body = CreateTokenResponse,
        ),
    ),
)]
#[post("")]
pub async fn create_connection_string_url(
    body: web::Json<CreateTokenBody>,
    profile: MyceliumProfileData,
    life_cycle_settings: web::Data<AccountLifeCycle>,
    sql_app_module: web::Data<SqlAppModule>,
) -> impl Responder {
    match create_connection_string(
        profile.to_profile(),
        body.expiration.to_owned(),
        body.tenant_id.to_owned(),
        body.service_account_id.to_owned(),
        body.roles.to_owned(),
        life_cycle_settings.get_ref().to_owned(),
        Box::new(&*sql_app_module.resolve_ref()),
        Box::new(&*sql_app_module.resolve_ref()),
    )
    .await
    {
        Ok(connection_string) => {
            HttpResponse::Ok().json(CreateTokenResponse { connection_string })
        }
        Err(err) => handle_mapped_error(err),
    }
}
