use crate::dtos::MyceliumProfileData;

use actix_web::{post, web, HttpResponse, Responder};
use myc_core::{
    models::AccountLifeCycle,
    use_cases::role_scoped::tenant_manager::create_tenant_associated_connection_string,
};
use myc_diesel::repositories::SqlAppModule;
use myc_http_tools::{
    utils::HttpJsonResponse,
    wrappers::default_response_to_http_response::handle_mapped_error,
    Permission,
};
use serde::{Deserialize, Serialize};
use shaku::HasComponent;
use utoipa::{ToResponse, ToSchema};
use uuid::Uuid;

// ? ---------------------------------------------------------------------------
// ? Configure application
// ? ---------------------------------------------------------------------------

pub fn configure(config: &mut web::ServiceConfig) {
    config.service(create_tenant_associated_connection_string_url);
}

// ? ---------------------------------------------------------------------------
// ? Define API structs
// ? ---------------------------------------------------------------------------

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateTenantScopedTokenBody {
    permissioned_roles: Vec<(String, Permission)>,
    expiration: i64,
}

#[derive(Serialize, ToSchema, ToResponse)]
#[serde(rename_all = "camelCase")]
pub struct CreateTokenResponse {
    connection_string: String,
}

/// Create Tenant Associated Token
///
#[utoipa::path(
    post,
    params(
        ("tenant_id" = Uuid, Path, description = "The tenant unique id."),
    ),
    request_body = CreateTenantScopedTokenBody,
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
#[post("/tenants/{tenant_id}")]
pub async fn create_tenant_associated_connection_string_url(
    path: web::Path<Uuid>,
    body: web::Json<CreateTenantScopedTokenBody>,
    profile: MyceliumProfileData,
    life_cycle_settings: web::Data<AccountLifeCycle>,
    sql_app_module: web::Data<SqlAppModule>,
) -> impl Responder {
    match create_tenant_associated_connection_string(
        profile.to_profile(),
        path.to_owned(),
        body.expiration.to_owned(),
        body.permissioned_roles.to_owned(),
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
