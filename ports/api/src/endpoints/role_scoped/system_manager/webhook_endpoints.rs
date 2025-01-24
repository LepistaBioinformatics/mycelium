use crate::dtos::MyceliumProfileData;

use actix_web::{delete, get, patch, post, web, Responder};
use myc_core::{
    domain::dtos::{
        http_secret::HttpSecret,
        webhook::{WebHook, WebHookTrigger},
    },
    models::AccountLifeCycle,
    use_cases::role_scoped::system_manager::webhook::{
        delete_webhook, list_webhooks, register_webhook, update_webhook,
    },
};
use myc_diesel::repositories::AppModule;
use myc_http_tools::{
    utils::HttpJsonResponse,
    wrappers::default_response_to_http_response::{
        create_response_kind, delete_response_kind, fetch_many_response_kind,
        handle_mapped_error, updating_response_kind,
    },
};
use serde::Deserialize;
use shaku::HasComponent;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

// ? ---------------------------------------------------------------------------
// ? Configure application
// ? ---------------------------------------------------------------------------

pub fn configure(config: &mut web::ServiceConfig) {
    config
        .service(crate_webhook_url)
        .service(list_webhooks_url)
        .service(update_webhook_url)
        .service(delete_webhook_url);
}

// ? ---------------------------------------------------------------------------
// ? Define API structs
// ? ---------------------------------------------------------------------------

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateWebHookBody {
    name: String,
    description: Option<String>,
    url: String,
    trigger: WebHookTrigger,
    secret: Option<HttpSecret>,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateWebHookBody {
    name: Option<String>,
    description: Option<String>,
    secret: Option<HttpSecret>,
}

#[derive(Deserialize, ToSchema, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct ListWebHooksParams {
    name: Option<String>,
    trigger: Option<WebHookTrigger>,
}

// ? ---------------------------------------------------------------------------
// ? Define endpoints
// ? ---------------------------------------------------------------------------

/// Create a webhook
#[utoipa::path(
    post,
    request_body = CreateWebHookBody,
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
            status = 201,
            description = "WebHook created.",
            body = WebHook,
        ),
        (
            status = 200,
            description = "WebHook already exists.",
            body = WebHook,
        ),
    ),
)]
#[post("")]
pub async fn crate_webhook_url(
    body: web::Json<CreateWebHookBody>,
    profile: MyceliumProfileData,
    life_cycle_settings: web::Data<AccountLifeCycle>,
    app_module: web::Data<AppModule>,
) -> impl Responder {
    match register_webhook(
        profile.to_profile(),
        body.name.to_owned(),
        body.description.to_owned(),
        body.url.to_owned(),
        body.trigger.to_owned(),
        body.secret.to_owned(),
        life_cycle_settings.get_ref().to_owned(),
        Box::new(&*app_module.resolve_ref()),
    )
    .await
    {
        Ok(res) => create_response_kind(res),
        Err(err) => handle_mapped_error(err),
    }
}

/// List webhooks
#[utoipa::path(
    get,
    params(
        ListWebHooksParams,
    ),
    responses(
        (
            status = 500,
            description = "Unknown internal server error.",
            body = HttpJsonResponse,
        ),
        (
            status = 204,
            description = "Not found.",
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
            status = 200,
            description = "Fetching success.",
            body = WebHook,
        ),
    ),
)]
#[get("")]
pub async fn list_webhooks_url(
    info: web::Query<ListWebHooksParams>,
    profile: MyceliumProfileData,
    app_module: web::Data<AppModule>,
) -> impl Responder {
    match list_webhooks(
        profile.to_profile(),
        info.name.to_owned(),
        info.trigger.to_owned(),
        Box::new(&*app_module.resolve_ref()),
    )
    .await
    {
        Ok(res) => fetch_many_response_kind(res),
        Err(err) => handle_mapped_error(err),
    }
}

/// Update a webhook
#[utoipa::path(
    patch,
    params(
        ("webhook_id" = Uuid, Path, description = "The webhook primary key."),
    ),
    request_body = UpdateWebHookBody,
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
            status = 202,
            description = "WebHook created.",
            body = WebHook,
        ),
    ),
)]
#[patch("/{webhook_id}")]
pub async fn update_webhook_url(
    body: web::Json<UpdateWebHookBody>,
    path: web::Path<Uuid>,
    profile: MyceliumProfileData,
    app_module: web::Data<AppModule>,
) -> impl Responder {
    match update_webhook(
        profile.to_profile(),
        path.to_owned(),
        body.name.to_owned(),
        body.description.to_owned(),
        body.secret.to_owned(),
        Box::new(&*app_module.resolve_ref()),
        Box::new(&*app_module.resolve_ref()),
    )
    .await
    {
        Ok(res) => updating_response_kind(res),
        Err(err) => handle_mapped_error(err),
    }
}

/// Delete a webhook
#[utoipa::path(
    delete,
    params(
        ("webhook_id" = Uuid, Path, description = "The webhook primary key."),
    ),
    responses(
        (
            status = 500,
            description = "Unknown internal server error.",
            body = HttpJsonResponse,
        ),
        (
            status = 400,
            description = "Webhook not deleted.",
            body = HttpJsonResponse,
        ),
        (
            status = 401,
            description = "Unauthorized.",
            body = HttpJsonResponse,
        ),
        (
            status = 403,
            description = "Forbidden.",
            body = HttpJsonResponse,
        ),
        (
            status = 204,
            description = "Webhook deleted.",
        ),
    ),
)]
#[delete("/{webhook_id}")]
pub async fn delete_webhook_url(
    path: web::Path<Uuid>,
    profile: MyceliumProfileData,
    app_module: web::Data<AppModule>,
) -> impl Responder {
    match delete_webhook(
        profile.to_profile(),
        path.to_owned(),
        Box::new(&*app_module.resolve_ref()),
    )
    .await
    {
        Ok(res) => delete_response_kind(res),
        Err(err) => handle_mapped_error(err),
    }
}
