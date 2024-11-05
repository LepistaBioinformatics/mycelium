use crate::{
    dtos::MyceliumProfileData,
    endpoints::{shared::UrlGroup, standard::shared::build_actor_context},
    modules::AccountDeletionModule,
};

use actix_web::{delete, web, Responder};
use myc_core::{
    domain::{actors::ActorName, entities::AccountDeletion},
    use_cases::roles::standard::tenant_manager::delete_subscription_account,
};
use myc_http_tools::wrappers::default_response_to_http_response::{
    delete_response_kind, handle_mapped_error,
};
use shaku_actix::Inject;
use uuid::Uuid;

// ? ---------------------------------------------------------------------------
// ? Configure application
// ? ---------------------------------------------------------------------------

pub fn configure(config: &mut web::ServiceConfig) {
    config.service(delete_subscription_account_url);
}

// ? ---------------------------------------------------------------------------
// ? Define API structs
// ? ---------------------------------------------------------------------------

// TODO

// ? ---------------------------------------------------------------------------
// ? Define API paths
// ? ---------------------------------------------------------------------------

#[utoipa::path(
    delete,
    context_path = build_actor_context(ActorName::TenantManager, UrlGroup::Accounts),
    params(
        ("tenant_id" = Uuid, Path, description = "The tenant primary key."),
        ("account_id" = Uuid, Path, description = "The account primary key."),
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
            description = "Account deleted.",
            body = HttpJsonResponse,
        ),
        (
            status = 204,
            description = "Account deleted.",
        ),
    ),
)]
#[delete("/{tenant_id}/accounts/{account_id}")]
pub async fn delete_subscription_account_url(
    path: web::Path<(Uuid, Uuid)>,
    profile: MyceliumProfileData,
    account_deletion_repo: Inject<AccountDeletionModule, dyn AccountDeletion>,
) -> impl Responder {
    let (tenant_id, account_id) = path.into_inner();

    match delete_subscription_account(
        profile.to_profile(),
        tenant_id,
        account_id,
        Box::new(&*account_deletion_repo),
    )
    .await
    {
        Ok(res) => delete_response_kind(res),
        Err(err) => handle_mapped_error(err),
    }
}
