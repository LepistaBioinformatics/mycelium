use crate::dtos::MyceliumProfileData;

use actix_web::{post, web, HttpResponse, Responder};
use myc_core::{
    models::AccountLifeCycle,
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
    expiration: i64,
}

#[derive(Serialize, ToSchema, ToResponse)]
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
