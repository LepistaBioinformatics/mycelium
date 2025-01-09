use crate::{
    dtos::{MyceliumProfileData, TenantData},
    modules::AccountRegistrationModule,
};

use actix_web::{post, web, Responder};
use myc_core::{
    domain::entities::AccountRegistration,
    use_cases::role_scoped::tenant_owner::create_management_account,
};
use myc_http_tools::{
    utils::HttpJsonResponse,
    wrappers::default_response_to_http_response::{
        get_or_create_response_kind, handle_mapped_error,
    },
    Account,
};
use shaku_actix::Inject;

// ? ---------------------------------------------------------------------------
// ? Configure application
// ? ---------------------------------------------------------------------------

pub fn configure(config: &mut web::ServiceConfig) {
    config.service(create_management_account_url);
}

// ? ---------------------------------------------------------------------------
// ? Define API paths
// ? ---------------------------------------------------------------------------

/// Create a management account.
#[utoipa::path(
    post,
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
#[post("")]
pub async fn create_management_account_url(
    tenant: TenantData,
    profile: MyceliumProfileData,
    account_registration_repo: Inject<
        AccountRegistrationModule,
        dyn AccountRegistration,
    >,
) -> impl Responder {
    match create_management_account(
        profile.to_profile(),
        tenant.tenant_id().to_owned(),
        Box::new(&*account_registration_repo),
    )
    .await
    {
        Ok(res) => get_or_create_response_kind(res),
        Err(err) => handle_mapped_error(err),
    }
}
