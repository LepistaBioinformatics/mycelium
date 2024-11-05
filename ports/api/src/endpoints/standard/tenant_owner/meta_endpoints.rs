use crate::{
    dtos::MyceliumProfileData,
    endpoints::{shared::UrlGroup, standard::shared::build_actor_context},
    modules::{
        TenantDeletionModule, TenantRegistrationModule, TenantUpdatingModule,
    },
};

use actix_web::{delete, patch, post, web, HttpResponse, Responder};
use myc_core::{
    domain::{
        actors::ActorName,
        dtos::tenant::TenantMetaKey,
        entities::{TenantDeletion, TenantRegistration, TenantUpdating},
    },
    use_cases::roles::standard::tenant_owner::{
        create_tenant_meta, delete_tenant_meta, update_tenant_meta,
    },
};
use myc_http_tools::{
    utils::HttpJsonResponse,
    wrappers::default_response_to_http_response::{
        create_response_kind, delete_response_kind, handle_mapped_error,
        updating_response_kind,
    },
};
use serde::Deserialize;
use shaku_actix::Inject;
use utoipa::ToSchema;
use uuid::Uuid;

// ? ---------------------------------------------------------------------------
// ? Configure application
// ? ---------------------------------------------------------------------------

pub fn configure(config: &mut web::ServiceConfig) {
    config
        .service(create_tenant_meta_url)
        .service(delete_tenant_meta_url)
        .service(update_tenant_meta_url);
}

// ? ---------------------------------------------------------------------------
// ? Define API structs
// ? ---------------------------------------------------------------------------

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateTenantMetaBody {
    key: TenantMetaKey,
    value: String,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeleteTenantMetaBody {
    key: TenantMetaKey,
}

// ? ---------------------------------------------------------------------------
// ? Define API paths
// ? ---------------------------------------------------------------------------

#[utoipa::path(
    post,
    context_path = build_actor_context(ActorName::TenantOwner, UrlGroup::Meta),
    params(
        ("tenant_id" = Uuid, Path, description = "The tenant primary key."),
    ),
    request_body = CreateTenantMetaBody,
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
            description = "Meta already exists.",
            body = HttpJsonResponse,
        ),
        (
            status = 201,
            description = "Meta created.",
            body = TenantMeta,
        ),
    ),
)]
#[post("/{tenant_id}")]
pub async fn create_tenant_meta_url(
    path: web::Path<Uuid>,
    body: web::Json<CreateTenantMetaBody>,
    profile: MyceliumProfileData,
    tenant_registration_repo: Inject<
        TenantRegistrationModule,
        dyn TenantRegistration,
    >,
) -> impl Responder {
    match create_tenant_meta(
        profile.to_profile(),
        path.into_inner(),
        body.key.to_owned(),
        body.value.to_owned(),
        Box::new(&*tenant_registration_repo),
    )
    .await
    {
        Ok(res) => create_response_kind(res),
        Err(err) => handle_mapped_error(err),
    }
}

#[utoipa::path(
    delete,
    context_path = build_actor_context(ActorName::TenantOwner, UrlGroup::Meta),
    params(
        ("tenant_id" = Uuid, Path, description = "The tenant primary key."),
    ),
    request_body = DeleteTenantMetaBody,
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
            description = "Meta not deleted.",
            body = HttpJsonResponse,
        ),
        (
            status = 204,
            description = "Meta deleted.",
        ),
    ),
)]
#[delete("/{tenant_id}")]
pub async fn delete_tenant_meta_url(
    path: web::Path<Uuid>,
    body: web::Json<DeleteTenantMetaBody>,
    profile: MyceliumProfileData,
    tenant_deletion_repo: Inject<TenantDeletionModule, dyn TenantDeletion>,
) -> impl Responder {
    match delete_tenant_meta(
        profile.to_profile(),
        path.to_owned(),
        body.key.to_owned(),
        Box::new(&*tenant_deletion_repo),
    )
    .await
    {
        Ok(res) => delete_response_kind(res),
        Err(err) => handle_mapped_error(err),
    }
}

#[utoipa::path(
    patch,
    context_path = build_actor_context(ActorName::TenantOwner, UrlGroup::Meta),
    params(
        ("tenant_id" = Uuid, Path, description = "The tenant primary key."),
    ),
    request_body = CreateTenantMetaBody,
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
            description = "Meta not updated.",
            body = HttpJsonResponse,
        ),
        (
            status = 202,
            description = "Meta updated.",
        ),
    ),
)]
#[patch("/{tenant_id}")]
pub async fn update_tenant_meta_url(
    path: web::Path<Uuid>,
    body: web::Json<CreateTenantMetaBody>,
    profile: MyceliumProfileData,
    tenant_updating_repo: Inject<TenantUpdatingModule, dyn TenantUpdating>,
) -> impl Responder {
    match update_tenant_meta(
        profile.to_profile(),
        path.to_owned(),
        body.key.to_owned(),
        body.value.to_owned(),
        Box::new(&*tenant_updating_repo),
    )
    .await
    {
        Ok(res) => updating_response_kind(res),
        Err(err) => handle_mapped_error(err),
    }
}
