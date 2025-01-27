use crate::dtos::{MyceliumProfileData, TenantData};

use actix_web::{delete, post, put, web, Responder};
use myc_core::{
    domain::dtos::tag::Tag,
    use_cases::role_scoped::subscriptions_manager::tag::{
        delete_tag, register_tag, update_tag,
    },
};
use myc_diesel::repositories::SqlAppModule;
use myc_http_tools::{
    utils::HttpJsonResponse,
    wrappers::default_response_to_http_response::{
        delete_response_kind, get_or_create_response_kind, handle_mapped_error,
        updating_response_kind,
    },
};
use serde::Deserialize;
use shaku::HasComponent;
use std::collections::HashMap;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

// ? ---------------------------------------------------------------------------
// ? Configure application
// ? ---------------------------------------------------------------------------

pub fn configure(config: &mut web::ServiceConfig) {
    config
        .service(register_account_tag_url)
        .service(update_account_tag_url)
        .service(delete_account_tag_url);
}

// ? ---------------------------------------------------------------------------
// ? Define API structs
// ? ---------------------------------------------------------------------------

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateAccountTagBody {
    account_id: Uuid,
    value: String,
    meta: HashMap<String, String>,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAccountTagBody {
    account_id: Uuid,
    value: String,
    meta: HashMap<String, String>,
}

#[derive(Deserialize, IntoParams, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeleteAccountTagParams {
    account_id: Uuid,
}

// ? ---------------------------------------------------------------------------
// ? Define API paths
//
// Account
//
// ? ---------------------------------------------------------------------------

/// Register a tag
#[utoipa::path(
    post,
    params(
        (
            "account_id" = Uuid,
            Path,
            description = "The account unique id."
        ),
        (
            "x-mycelium-tenant-id" = Uuid,
            Header,
            description = "The tenant unique id."
        ),
    ),
    request_body = CreateAccountTagBody,
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
            description = "Tag successfully registered.",
            body = Tag,
        ),
    ),
)]
#[post("")]
pub async fn register_account_tag_url(
    tenant: TenantData,
    profile: MyceliumProfileData,
    body: web::Json<CreateAccountTagBody>,
    app_module: web::Data<SqlAppModule>,
) -> impl Responder {
    match register_tag(
        profile.to_profile(),
        tenant.tenant_id().to_owned(),
        body.account_id,
        body.value.to_owned(),
        body.meta.to_owned(),
        Box::new(&*app_module.resolve_ref()),
    )
    .await
    {
        Ok(res) => get_or_create_response_kind(res),
        Err(err) => handle_mapped_error(err),
    }
}

/// Update a tag
#[utoipa::path(
    put,
    params(
        (
            "tag_id" = Uuid,
            Path,
            description = "The tag primary key."
        ),
        (
            "x-mycelium-tenant-id" = Uuid,
            Header,
            description = "The tenant unique id."
        ),
    ),
    request_body = UpdateAccountTagBody,
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
            description = "Tag successfully registered.",
            body = Tag,
        ),
    ),
)]
#[put("/{tag_id}")]
pub async fn update_account_tag_url(
    tenant: TenantData,
    profile: MyceliumProfileData,
    path: web::Path<Uuid>,
    body: web::Json<UpdateAccountTagBody>,
    app_module: web::Data<SqlAppModule>,
) -> impl Responder {
    match update_tag(
        profile.to_profile(),
        tenant.tenant_id().to_owned(),
        body.account_id,
        Tag {
            id: path.into_inner(),
            value: body.value.to_owned(),
            meta: Some(body.meta.to_owned()),
        },
        Box::new(&*app_module.resolve_ref()),
    )
    .await
    {
        Ok(res) => updating_response_kind(res),
        Err(err) => handle_mapped_error(err),
    }
}

/// Delete a tag
#[utoipa::path(
    delete,
    params(
        (
            "tag_id" = Uuid,
            Path,
            description = "The tag primary key."
        ),
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
            description = "Bad request.",
            body = HttpJsonResponse,
        ),
        (
            status = 201,
            description = "Tag successfully registered.",
            body = Tag,
        ),
    ),
)]
#[delete("/{tag_id}")]
pub async fn delete_account_tag_url(
    tenant: TenantData,
    profile: MyceliumProfileData,
    path: web::Path<Uuid>,
    query: web::Query<DeleteAccountTagParams>,
    app_module: web::Data<SqlAppModule>,
) -> impl Responder {
    match delete_tag(
        profile.to_profile(),
        tenant.tenant_id().to_owned(),
        query.account_id,
        path.into_inner(),
        Box::new(&*app_module.resolve_ref()),
    )
    .await
    {
        Ok(res) => delete_response_kind(res),
        Err(err) => handle_mapped_error(err),
    }
}
