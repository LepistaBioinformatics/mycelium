use crate::{
    dtos::{MyceliumProfileData, TenantData},
    endpoints::{shared::UrlGroup, standard::shared::build_actor_context},
    modules::{
        TenantDeletionModule, TenantFetchingModule, TenantUpdatingModule,
        UserFetchingModule,
    },
};

use actix_web::{delete, post, web, HttpResponse, Responder};
use myc_core::{
    domain::{
        actors::ActorName,
        entities::{
            TenantDeletion, TenantFetching, TenantUpdating, UserFetching,
        },
    },
    use_cases::roles::standard::tenant_owner::{
        guest_tenant_owner, revoke_tenant_owner,
    },
};
use myc_http_tools::{
    utils::HttpJsonResponse,
    wrappers::default_response_to_http_response::{
        create_response_kind, delete_response_kind, handle_mapped_error,
    },
    Email,
};
use serde::Deserialize;
use shaku_actix::Inject;
use utoipa::ToSchema;

// ? ---------------------------------------------------------------------------
// ? Configure application
// ? ---------------------------------------------------------------------------

pub fn configure(config: &mut web::ServiceConfig) {
    config
        .service(guest_tenant_owner_url)
        .service(revoke_tenant_owner_url);
}

// ? ---------------------------------------------------------------------------
// ? Define API structs
// ? ---------------------------------------------------------------------------

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GuestTenantOwnerBody {
    email: String,
}

// ? ---------------------------------------------------------------------------
// ? Define API paths
// ? ---------------------------------------------------------------------------

#[utoipa::path(
    post,
    context_path = build_actor_context(ActorName::TenantOwner, UrlGroup::Owners),
    params(
        (
            "x-mycelium-tenant-id" = TenantData,
            Header,
            description = "The tenant unique id."
        ),
    ),
    request_body = GuestTenantOwnerBody,
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
            description = "Owner already exists.",
            body = HttpJsonResponse,
        ),
        (
            status = 201,
            description = "Owner created.",
            body = TenantOwnerConnection,
        ),
    ),
)]
#[post("/")]
pub async fn guest_tenant_owner_url(
    tenant: TenantData,
    body: web::Json<GuestTenantOwnerBody>,
    profile: MyceliumProfileData,
    owner_fetching_repo: Inject<UserFetchingModule, dyn UserFetching>,
    tenant_updating_repo: Inject<TenantUpdatingModule, dyn TenantUpdating>,
) -> impl Responder {
    let email = match Email::from_string(body.email.to_owned()) {
        Ok(email) => email,
        Err(err) => {
            return HttpResponse::BadRequest()
                .json(HttpJsonResponse::new_message(err.to_string()))
        }
    };

    match guest_tenant_owner(
        profile.to_profile(),
        email,
        tenant.tenant_id().to_owned(),
        Box::new(&*owner_fetching_repo),
        Box::new(&*tenant_updating_repo),
    )
    .await
    {
        Ok(res) => create_response_kind(res),
        Err(err) => handle_mapped_error(err),
    }
}

#[utoipa::path(
    delete,
    context_path = build_actor_context(ActorName::TenantOwner, UrlGroup::Owners),
    params(
        (
            "x-mycelium-tenant-id" = TenantData,
            Header,
            description = "The tenant unique id."
        ),
    ),
    request_body = GuestTenantOwnerBody,
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
            description = "Owner deleted.",
            body = HttpJsonResponse,
        ),
        (
            status = 204,
            description = "Owner deleted.",
        ),
    ),
)]
#[delete("/")]
pub async fn revoke_tenant_owner_url(
    tenant: TenantData,
    body: web::Json<GuestTenantOwnerBody>,
    profile: MyceliumProfileData,
    tenant_fetching_repo: Inject<TenantFetchingModule, dyn TenantFetching>,
    tenant_deletion_repo: Inject<TenantDeletionModule, dyn TenantDeletion>,
) -> impl Responder {
    let email = match Email::from_string(body.email.to_owned()) {
        Ok(email) => email,
        Err(err) => {
            return HttpResponse::BadRequest()
                .json(HttpJsonResponse::new_message(err.to_string()))
        }
    };

    match revoke_tenant_owner(
        profile.to_profile(),
        email,
        tenant.tenant_id().to_owned(),
        Box::new(&*tenant_fetching_repo),
        Box::new(&*tenant_deletion_repo),
    )
    .await
    {
        Ok(res) => delete_response_kind(res),
        Err(err) => handle_mapped_error(err),
    }
}
