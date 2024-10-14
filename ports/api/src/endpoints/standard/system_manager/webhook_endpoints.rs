use crate::{
    dtos::MyceliumProfileData,
    endpoints::{shared::UrlGroup, standard::shared::build_actor_context},
    modules::{
        WebHookDeletionModule, WebHookFetchingModule,
        WebHookRegistrationModule, WebHookUpdatingModule,
    },
};

use actix_web::{delete, get, post, put, web, HttpResponse, Responder};
use myc_core::{
    domain::{
        actors::ActorName,
        dtos::webhook::{HookTarget, WebHook},
        entities::{
            WebHookDeletion, WebHookFetching, WebHookRegistration,
            WebHookUpdating,
        },
    },
    use_cases::roles::{
        shared::webhook::default_actions::WebHookDefaultAction,
        standard::system_manager::webhook::{
            delete_webhook, list_webhooks, register_webhook, update_webhook,
        },
    },
};
use myc_http_tools::utils::JsonError;
use mycelium_base::entities::{
    CreateResponseKind, DeletionResponseKind, FetchManyResponseKind,
    UpdatingResponseKind,
};
use serde::Deserialize;
use shaku_actix::Inject;
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
    url: String,
    action: WebHookDefaultAction,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateWebHookBody {
    webhook: WebHook,
}

#[derive(Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct ListWebHooksParams {
    name: Option<String>,
    target: Option<HookTarget>,
}

// ? ---------------------------------------------------------------------------
// ? Define endpoints
// ? ---------------------------------------------------------------------------

#[utoipa::path(
    post,
    context_path = build_actor_context(ActorName::SystemManager, UrlGroup::Webhooks),
    request_body = CreateWebHookBody,
    responses(
        (
            status = 500,
            description = "Unknown internal server error.",
            body = JsonError,
        ),
        (
            status = 403,
            description = "Forbidden.",
            body = JsonError,
        ),
        (
            status = 401,
            description = "Unauthorized.",
            body = JsonError,
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
#[post("/")]
pub async fn crate_webhook_url(
    body: web::Json<CreateWebHookBody>,
    profile: MyceliumProfileData,
    webhook_registration_repo: Inject<
        WebHookRegistrationModule,
        dyn WebHookRegistration,
    >,
) -> impl Responder {
    match register_webhook(
        profile.to_profile(),
        body.url.to_owned(),
        body.action.to_owned(),
        Box::new(&*webhook_registration_repo),
    )
    .await
    {
        Err(err) => HttpResponse::InternalServerError()
            .json(JsonError::new(err.to_string())),
        Ok(res) => match res {
            CreateResponseKind::NotCreated(guest, _) => {
                HttpResponse::Ok().json(guest)
            }
            CreateResponseKind::Created(guest) => {
                HttpResponse::Created().json(guest)
            }
        },
    }
}

#[utoipa::path(
    get,
    context_path = build_actor_context(ActorName::SystemManager, UrlGroup::Webhooks),
    params(
        ListWebHooksParams,
    ),
    responses(
        (
            status = 500,
            description = "Unknown internal server error.",
            body = JsonError,
        ),
        (
            status = 204,
            description = "Not found.",
        ),
        (
            status = 403,
            description = "Forbidden.",
            body = JsonError,
        ),
        (
            status = 401,
            description = "Unauthorized.",
            body = JsonError,
        ),
        (
            status = 200,
            description = "Fetching success.",
            body = Webhook,
        ),
    ),
)]
#[get("/")]
pub async fn list_webhooks_url(
    info: web::Query<ListWebHooksParams>,
    profile: MyceliumProfileData,
    webhook_fetching_repo: Inject<WebHookFetchingModule, dyn WebHookFetching>,
) -> impl Responder {
    match list_webhooks(
        profile.to_profile(),
        info.name.to_owned(),
        info.target.to_owned(),
        Box::new(&*webhook_fetching_repo),
    )
    .await
    {
        Err(err) => HttpResponse::InternalServerError()
            .json(JsonError::new(err.to_string())),
        Ok(res) => match res {
            FetchManyResponseKind::NotFound => {
                HttpResponse::NotFound().finish()
            }
            FetchManyResponseKind::Found(guest) => {
                HttpResponse::Ok().json(guest)
            }
            FetchManyResponseKind::FoundPaginated(guest) => {
                HttpResponse::Ok().json(guest)
            }
        },
    }
}

#[utoipa::path(
    put,
    context_path = build_actor_context(ActorName::SystemManager, UrlGroup::Webhooks),
    params(
        ("id" = Uuid, Path, description = "The webhook primary key."),
    ),
    request_body = UpdateWebHookBody,
    responses(
        (
            status = 500,
            description = "Unknown internal server error.",
            body = JsonError,
        ),
        (
            status = 403,
            description = "Forbidden.",
            body = JsonError,
        ),
        (
            status = 401,
            description = "Unauthorized.",
            body = JsonError,
        ),
        (
            status = 202,
            description = "WebHook created.",
            body = WebHook,
        ),
    ),
)]
#[put("/{id}")]
pub async fn update_webhook_url(
    body: web::Json<UpdateWebHookBody>,
    path: web::Path<Uuid>,
    profile: MyceliumProfileData,
    webhook_updating_repo: Inject<WebHookUpdatingModule, dyn WebHookUpdating>,
) -> impl Responder {
    match update_webhook(
        profile.to_profile(),
        body.webhook.to_owned(),
        path.to_owned(),
        Box::new(&*webhook_updating_repo),
    )
    .await
    {
        Err(err) => HttpResponse::InternalServerError()
            .json(JsonError::new(err.to_string())),
        Ok(res) => match res {
            UpdatingResponseKind::NotUpdated(_, msg) => {
                HttpResponse::BadRequest().json(JsonError::new(msg))
            }
            UpdatingResponseKind::Updated(webhook) => {
                HttpResponse::Accepted().json(webhook)
            }
        },
    }
}

#[utoipa::path(
    delete,
    context_path = build_actor_context(ActorName::SystemManager, UrlGroup::Webhooks),
    params(
        ("id" = Uuid, Path, description = "The webhook primary key."),
    ),
    responses(
        (
            status = 500,
            description = "Unknown internal server error.",
            body = JsonError,
        ),
        (
            status = 400,
            description = "Webhook not deleted.",
            body = JsonError,
        ),
        (
            status = 401,
            description = "Unauthorized.",
            body = JsonError,
        ),
        (
            status = 403,
            description = "Forbidden.",
            body = JsonError,
        ),
        (
            status = 204,
            description = "Webhook deleted.",
        ),
    ),
)]
#[delete("/{id}/delete")]
pub async fn delete_webhook_url(
    path: web::Path<Uuid>,
    profile: MyceliumProfileData,
    webhook_deletion_repo: Inject<WebHookDeletionModule, dyn WebHookDeletion>,
) -> impl Responder {
    match delete_webhook(
        profile.to_profile(),
        path.to_owned(),
        Box::new(&*webhook_deletion_repo),
    )
    .await
    {
        Err(err) => HttpResponse::InternalServerError()
            .json(JsonError::new(err.to_string())),
        Ok(res) => match res {
            DeletionResponseKind::NotDeleted(_, msg) => {
                HttpResponse::BadRequest().json(JsonError::new(msg))
            }
            DeletionResponseKind::Deleted => HttpResponse::NoContent().finish(),
        },
    }
}
