use crate::{
    dtos::MyceliumProfileData,
    endpoints::{shared::UrlGroup, standard::shared::build_actor_context},
    modules::{AccountRegistrationModule, TenantFetchingModule},
};

use actix_web::{post, web, HttpResponse, Responder};
use myc_core::{
    domain::{
        actors::ActorName,
        entities::{AccountRegistration, TenantFetching},
    },
    use_cases::roles::standard::tenant_owner::create_management_account,
};
use myc_http_tools::{
    utils::HttpJsonResponse,
    wrappers::default_response_to_http_response::{
        create_response_kind, handle_mapped_error,
    },
};
use shaku_actix::Inject;
use uuid::Uuid;

// ? ---------------------------------------------------------------------------
// ? Configure application
// ? ---------------------------------------------------------------------------

pub fn configure(config: &mut web::ServiceConfig) {
    config.service(create_management_account_url);
}

// ? ---------------------------------------------------------------------------
// ? Define API structs
// ? ---------------------------------------------------------------------------

// TODO

// ? ---------------------------------------------------------------------------
// ? Define API paths
// ? ---------------------------------------------------------------------------

#[utoipa::path(
    post,
    context_path = build_actor_context(ActorName::TenantOwner, UrlGroup::Accounts),
    params(
        ("tenant_id" = Uuid, Path, description = "The tenant primary key."),
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
            body = CreateSubscriptionResponse,
        ),
    ),
)]
#[post("/{tenant_id}")]
pub async fn create_management_account_url(
    path: web::Path<Uuid>,
    profile: MyceliumProfileData,
    tenant_fetching_repo: Inject<TenantFetchingModule, dyn TenantFetching>,
    account_registration_repo: Inject<
        AccountRegistrationModule,
        dyn AccountRegistration,
    >,
) -> impl Responder {
    match create_management_account(
        profile.to_profile(),
        path.into_inner(),
        Box::new(&*tenant_fetching_repo),
        Box::new(&*account_registration_repo),
    )
    .await
    {
        Ok(res) => create_response_kind(res),
        Err(err) => handle_mapped_error(err),
    }
}
