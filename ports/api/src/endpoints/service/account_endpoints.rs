use crate::{
    dtos::MyceliumTenantScopedConnectionStringData,
    modules::{AccountRegistrationModule, WebHookFetchingModule},
};

use actix_web::{post, web, HttpResponse, Responder};
use myc_core::{
    domain::entities::{AccountRegistration, WebHookFetching},
    models::AccountLifeCycle,
    use_cases::service::account::create_subscription_account,
};
use myc_http_tools::{
    utils::HttpJsonResponse,
    wrappers::default_response_to_http_response::handle_mapped_error, Account,
};
use serde::Deserialize;
use shaku_actix::Inject;
use utoipa::ToSchema;

// ? ---------------------------------------------------------------------------
// ? Configure application
// ? ---------------------------------------------------------------------------

pub fn configure(config: &mut web::ServiceConfig) {
    config.service(
        web::scope("/accounts")
            .service(create_subscription_account_from_service_url),
    );
}

// ? ---------------------------------------------------------------------------
// ? Define API structs
// ? ---------------------------------------------------------------------------

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateSubscriptionAccountBody {
    name: String,
}

// ? ---------------------------------------------------------------------------
// ? Define API paths
// ? ---------------------------------------------------------------------------

/// Create Subscription Account
///
/// Subscription accounts represents shared entities, like institutions,
/// groups, but not real persons.
#[utoipa::path(
    post,
    params(
        (
            "x-mycelium-connection-string" = String,
            Header,
            description = "The connection string to the role-scoped database."
        ),
    ),
    request_body = CreateSubscriptionAccountBody,
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
            description = "Account already exists.",
            body = HttpJsonResponse,
        ),
        (
            status = 201,
            description = "Account created.",
            body = Account,
        ),
    ),
    security(("ConnectionString" = [])),
)]
#[post("")]
pub async fn create_subscription_account_from_service_url(
    body: web::Json<CreateSubscriptionAccountBody>,
    connection_string: MyceliumTenantScopedConnectionStringData,
    life_cycle_settings: web::Data<AccountLifeCycle>,
    account_registration_repo: Inject<
        AccountRegistrationModule,
        dyn AccountRegistration,
    >,
    webhook_fetching_repo: Inject<WebHookFetchingModule, dyn WebHookFetching>,
) -> impl Responder {
    let tenant_id = match connection_string.tenant_id() {
        Some(tenant_id) => tenant_id,
        None => {
            return HttpResponse::BadRequest()
                .json("Tenant id not found in the connection string.");
        }
    };

    match create_subscription_account(
        connection_string.connection_string().clone(),
        tenant_id,
        body.name.to_owned(),
        life_cycle_settings.get_ref().clone(),
        Box::new(&*account_registration_repo),
        Box::new(&*webhook_fetching_repo),
    )
    .await
    {
        Err(err) => handle_mapped_error(err),
        Ok(account) => HttpResponse::Created().json(account),
    }
}
