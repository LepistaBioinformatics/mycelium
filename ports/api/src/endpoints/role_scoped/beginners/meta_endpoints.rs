use std::str::FromStr;

use crate::dtos::MyceliumProfileData;

use actix_web::{delete, post, put, web, HttpResponse, Responder};
use myc_core::{
    domain::dtos::account::{AccountMeta, AccountMetaKey},
    use_cases::role_scoped::beginner::meta::{
        create_account_meta, delete_account_meta, update_account_meta,
    },
};
use myc_diesel::repositories::SqlAppModule;
use myc_http_tools::{
    utils::HttpJsonResponse,
    wrappers::default_response_to_http_response::{
        create_response_kind, delete_response_kind, handle_mapped_error,
        updating_response_kind,
    },
};
use serde::Deserialize;
use shaku::HasComponent;
use utoipa::{IntoParams, ToSchema};

// ? ---------------------------------------------------------------------------
// ? Configure application
// ? ---------------------------------------------------------------------------

pub fn configure(config: &mut web::ServiceConfig) {
    config
        .service(create_account_meta_url)
        .service(update_account_meta_url)
        .service(delete_account_meta_url);
}

// ? ---------------------------------------------------------------------------
// ? Define API structs
// ? ---------------------------------------------------------------------------

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateAccountMetaBody {
    key: String,
    value: String,
}

#[derive(Deserialize, ToSchema, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct DeleteAccountMetaParams {
    key: String,
}

// ? ---------------------------------------------------------------------------
// ? Define API paths
// ? ---------------------------------------------------------------------------

/// Register a account metadata
#[utoipa::path(
    post,
    operation_id = "create_account_meta",
    request_body = CreateAccountMetaBody,
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
            body = AccountMeta,
        ),
    ),
)]
#[post("")]
pub async fn create_account_meta_url(
    body: web::Json<CreateAccountMetaBody>,
    profile: MyceliumProfileData,
    app_module: web::Data<SqlAppModule>,
) -> impl Responder {
    let key = match AccountMetaKey::from_str(&body.key) {
        Ok(key) => key,
        Err(_) => {
            return HttpResponse::BadRequest().json(
                HttpJsonResponse::new_message("The key is invalid".to_string()),
            );
        }
    };

    match create_account_meta(
        profile.to_profile(),
        key.to_owned(),
        body.value.to_owned(),
        Box::new(&*app_module.resolve_ref()),
    )
    .await
    {
        Ok(res) => create_response_kind(res),
        Err(err) => handle_mapped_error(err),
    }
}

/// Update a account metadata
#[utoipa::path(
    put,
    operation_id = "update_account_meta",
    request_body = CreateAccountMetaBody,
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
#[put("")]
pub async fn update_account_meta_url(
    body: web::Json<CreateAccountMetaBody>,
    profile: MyceliumProfileData,
    app_module: web::Data<SqlAppModule>,
) -> impl Responder {
    let key = match AccountMetaKey::from_str(&body.key) {
        Ok(key) => key,
        Err(_) => {
            return HttpResponse::BadRequest().json(
                HttpJsonResponse::new_message("The key is invalid".to_string()),
            );
        }
    };

    match update_account_meta(
        profile.to_profile(),
        key.to_owned(),
        body.value.to_owned(),
        Box::new(&*app_module.resolve_ref()),
    )
    .await
    {
        Ok(res) => updating_response_kind(res),
        Err(err) => handle_mapped_error(err),
    }
}

/// Delete a account metadata
#[utoipa::path(
    delete,
    operation_id = "delete_account_meta",
    request_body = DeleteAccountMetaParams,
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
#[delete("")]
pub async fn delete_account_meta_url(
    query: web::Query<DeleteAccountMetaParams>,
    profile: MyceliumProfileData,
    app_module: web::Data<SqlAppModule>,
) -> impl Responder {
    let key = match AccountMetaKey::from_str(&query.key) {
        Ok(key) => key,
        Err(_) => {
            return HttpResponse::BadRequest().json(
                HttpJsonResponse::new_message("The key is invalid".to_string()),
            );
        }
    };

    match delete_account_meta(
        profile.to_profile(),
        key.to_owned(),
        Box::new(&*app_module.resolve_ref()),
    )
    .await
    {
        Ok(res) => delete_response_kind(res),
        Err(err) => handle_mapped_error(err),
    }
}
