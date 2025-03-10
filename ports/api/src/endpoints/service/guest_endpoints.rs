use crate::dtos::MyceliumRoleScopedConnectionStringData;

use actix_web::{post, web, HttpResponse, Responder};
use myc_core::{
    domain::dtos::{account::Account, user::User},
    models::AccountLifeCycle,
    use_cases::service::guest::guest_to_default_account,
};
use myc_diesel::repositories::SqlAppModule;
use myc_http_tools::{
    utils::HttpJsonResponse,
    wrappers::default_response_to_http_response::handle_mapped_error,
};
use mycelium_base::dtos::Children;
use serde::Deserialize;
use shaku::HasComponent;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

// ? ---------------------------------------------------------------------------
// ? Configure application
// ? ---------------------------------------------------------------------------

pub fn configure(config: &mut web::ServiceConfig) {
    config.service(web::scope("/guests").service(guest_to_default_account_url));
}

// ? ---------------------------------------------------------------------------
// ? Define API structs
// ? ---------------------------------------------------------------------------

#[derive(Deserialize, ToSchema, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct ServiceGuestUserBody {
    #[serde(flatten)]
    account: Account,
}

// ? ---------------------------------------------------------------------------
// ? Define API paths
//
// Guest
//
// ? ---------------------------------------------------------------------------

/// Guest a user to work on account.
///
/// This action gives the ability of the target account (specified through
/// the `account` argument) to perform actions specified in the `role`
/// path argument.
#[utoipa::path(
    post,
    params(
        ("role_id" = Uuid, Path, description = "The guest-role unique id."),
        (
            "x-mycelium-connection-string" = String,
            Header,
            description = "The connection string to the role-scoped database."
        ),
    ),
    request_body = ServiceGuestUserBody,
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
            description = "Guesting done.",
            body = Account,
        ),
        (
            status = 200,
            description = "Guest already exist.",
            body = Account,
        ),
    ),
    security(("ConnectionString" = [])),
)]
#[post("/roles/{role_id}")]
pub async fn guest_to_default_account_url(
    path: web::Path<Uuid>,
    connection_string: MyceliumRoleScopedConnectionStringData,
    body: web::Json<ServiceGuestUserBody>,
    life_cycle_settings: web::Data<AccountLifeCycle>,
    sql_app_module: web::Data<SqlAppModule>,
) -> impl Responder {
    let role_id = path.to_owned();

    let email = match body.account.owners.to_owned() {
        Children::Ids(_) => {
            return HttpResponse::BadRequest()
                .json("Invalid account owner".to_string())
        }
        Children::Records(owners) => owners
            .into_iter()
            .filter(|owner| owner.is_principal())
            .collect::<Vec<User>>()
            .first()
            .unwrap()
            .email
            .to_owned(),
    };

    let tenant_id = match connection_string.tenant_id() {
        Some(tenant_id) => tenant_id,
        None => {
            return HttpResponse::BadRequest()
                .json("Tenant id not found in the connection string.");
        }
    };

    match guest_to_default_account(
        connection_string.connection_string().clone(),
        role_id,
        email.to_owned(),
        tenant_id,
        life_cycle_settings.get_ref().to_owned(),
        Box::new(&*sql_app_module.resolve_ref()),
        Box::new(&*sql_app_module.resolve_ref()),
        Box::new(&*sql_app_module.resolve_ref()),
        Box::new(&*sql_app_module.resolve_ref()),
    )
    .await
    {
        Ok(_) => HttpResponse::Created().json(email),
        Err(err) => handle_mapped_error(err),
    }
}
