use crate::{
    dtos::{MyceliumProfileData, TenantData},
    endpoints::shared::{build_actor_context, UrlGroup},
    modules::{AccountRegistrationModule, TenantFetchingModule},
};

use actix_web::{post, web, Responder};
use myc_core::{
    domain::{
        actors::ActorName,
        entities::{AccountRegistration, TenantFetching},
    },
    use_cases::roles::standard::tenant_owner::create_management_account,
};
use myc_http_tools::wrappers::default_response_to_http_response::{
    create_response_kind, handle_mapped_error,
};
use shaku_actix::Inject;

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
        (
            "x-mycelium-tenant-id" = Uuid,
            Header,
            description = "The tenant unique id."
        ),
    ),
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
)]
#[post("/")]
pub async fn create_management_account_url(
    tenant: TenantData,
    profile: MyceliumProfileData,
    tenant_fetching_repo: Inject<TenantFetchingModule, dyn TenantFetching>,
    account_registration_repo: Inject<
        AccountRegistrationModule,
        dyn AccountRegistration,
    >,
) -> impl Responder {
    match create_management_account(
        profile.to_profile(),
        tenant.tenant_id().to_owned(),
        Box::new(&*tenant_fetching_repo),
        Box::new(&*account_registration_repo),
    )
    .await
    {
        Ok(res) => create_response_kind(res),
        Err(err) => handle_mapped_error(err),
    }
}
