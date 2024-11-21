use crate::{
    dtos::{MyceliumProfileData, TenantData},
    endpoints::{shared::UrlGroup, standard::shared::build_actor_context},
    modules::TenantUpdatingModule,
};

use actix_web::{patch, web, Responder};
use myc_core::{
    domain::{actors::ActorName, entities::TenantUpdating},
    use_cases::roles::standard::tenant_owner::{
        update_tenant_archiving_status, update_tenant_name_and_description,
        update_tenant_trashing_status, update_tenant_verifying_status,
    },
};
use myc_http_tools::wrappers::default_response_to_http_response::{
    handle_mapped_error, updating_response_kind,
};
use serde::Deserialize;
use shaku_actix::Inject;
use utoipa::ToSchema;

// ? ---------------------------------------------------------------------------
// ? Configure application
// ? ---------------------------------------------------------------------------

pub fn configure(config: &mut web::ServiceConfig) {
    config
        .service(update_tenant_name_and_description_url)
        .service(update_tenant_archiving_status_url)
        .service(update_tenant_trashing_status_url)
        .service(update_tenant_verifying_status_url);
}

// ? ---------------------------------------------------------------------------
// ? Define API structs
// ? ---------------------------------------------------------------------------

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTenantNameAndDescriptionBody {
    name: Option<String>,
    description: Option<String>,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTenantArchivingBody {
    archived: bool,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTenantTrashingBody {
    trashed: bool,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTenantVerifyingBody {
    verified: bool,
}

// ? ---------------------------------------------------------------------------
// ? Define API paths
// ? ---------------------------------------------------------------------------

#[utoipa::path(
    patch,
    context_path = build_actor_context(ActorName::TenantOwner, UrlGroup::Tenants),
    params(
        (
            "x-mycelium-tenant-id" = TenantData,
            Header,
            description = "The tenant unique id."
        ),
    ),
    request_body = UpdateTenantNameAndDescriptionBody,
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
            description = "Tenant not updated.",
            body = HttpJsonResponse,
        ),
        (
            status = 202,
            description = "Tenant updated.",
        ),
    ),
)]
#[patch("/")]
pub async fn update_tenant_name_and_description_url(
    tenant: TenantData,
    body: web::Json<UpdateTenantNameAndDescriptionBody>,
    profile: MyceliumProfileData,
    tenant_updating_repo: Inject<TenantUpdatingModule, dyn TenantUpdating>,
) -> impl Responder {
    match update_tenant_name_and_description(
        profile.to_profile(),
        tenant.tenant_id().to_owned(),
        body.name.to_owned(),
        body.description.to_owned(),
        Box::new(&*tenant_updating_repo),
    )
    .await
    {
        Ok(res) => updating_response_kind(res),
        Err(err) => handle_mapped_error(err),
    }
}

#[utoipa::path(
    patch,
    context_path = build_actor_context(ActorName::TenantOwner, UrlGroup::Tenants),
    params(
        (
            "x-mycelium-tenant-id" = TenantData,
            Header,
            description = "The tenant unique id."
        ),
    ),
    request_body = UpdateTenantArchivingBody,
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
            description = "Tenant not updated.",
            body = HttpJsonResponse,
        ),
        (
            status = 202,
            description = "Tenant updated.",
        ),
    ),
)]
#[patch("/archive")]
pub async fn update_tenant_archiving_status_url(
    tenant: TenantData,
    body: web::Json<UpdateTenantArchivingBody>,
    profile: MyceliumProfileData,
    tenant_updating_repo: Inject<TenantUpdatingModule, dyn TenantUpdating>,
) -> impl Responder {
    match update_tenant_archiving_status(
        profile.to_profile(),
        tenant.tenant_id().to_owned(),
        body.archived.to_owned(),
        Box::new(&*tenant_updating_repo),
    )
    .await
    {
        Ok(res) => updating_response_kind(res),
        Err(err) => handle_mapped_error(err),
    }
}

#[utoipa::path(
    patch,
    context_path = build_actor_context(ActorName::TenantOwner, UrlGroup::Tenants),
    params(
        (
            "x-mycelium-tenant-id" = TenantData,
            Header,
            description = "The tenant unique id."
        ),
    ),
    request_body = UpdateTenantTrashingBody,
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
            description = "Tenant not updated.",
            body = HttpJsonResponse,
        ),
        (
            status = 202,
            description = "Tenant updated.",
        ),
    ),
)]
#[patch("/trash")]
pub async fn update_tenant_trashing_status_url(
    tenant: TenantData,
    body: web::Json<UpdateTenantTrashingBody>,
    profile: MyceliumProfileData,
    tenant_updating_repo: Inject<TenantUpdatingModule, dyn TenantUpdating>,
) -> impl Responder {
    match update_tenant_trashing_status(
        profile.to_profile(),
        tenant.tenant_id().to_owned(),
        body.trashed.to_owned(),
        Box::new(&*tenant_updating_repo),
    )
    .await
    {
        Ok(res) => updating_response_kind(res),
        Err(err) => handle_mapped_error(err),
    }
}

#[utoipa::path(
    patch,
    context_path = build_actor_context(ActorName::TenantOwner, UrlGroup::Tenants),
    params(
        (
            "x-mycelium-tenant-id" = TenantData,
            Header,
            description = "The tenant unique id."
        ),
    ),
    request_body = UpdateTenantVerifyingBody,
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
            description = "Tenant not updated.",
            body = HttpJsonResponse,
        ),
        (
            status = 202,
            description = "Tenant updated.",
        ),
    ),
)]
#[patch("/verify")]
pub async fn update_tenant_verifying_status_url(
    tenant: TenantData,
    body: web::Json<UpdateTenantVerifyingBody>,
    profile: MyceliumProfileData,
    tenant_updating_repo: Inject<TenantUpdatingModule, dyn TenantUpdating>,
) -> impl Responder {
    match update_tenant_verifying_status(
        profile.to_profile(),
        tenant.tenant_id().to_owned(),
        body.verified.to_owned(),
        Box::new(&*tenant_updating_repo),
    )
    .await
    {
        Ok(res) => updating_response_kind(res),
        Err(err) => handle_mapped_error(err),
    }
}
