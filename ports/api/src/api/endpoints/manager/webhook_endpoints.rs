use crate::modules::{WebHookDeletionModule, WebHookRegistrationModule};

use actix_web::{delete, post, web, HttpResponse, Responder};
use clean_base::entities::{CreateResponseKind, DeletionResponseKind};
use myc_core::{
    domain::entities::{WebHookDeletion, WebHookRegistration},
    use_cases::roles::{
        managers::webhook::{delete_webhook, register_webhook},
        shared::webhook::default_actions::WebHookDefaultAction,
    },
};
use myc_http_tools::{middleware::MyceliumProfileData, utils::JsonError};
use serde::Deserialize;
use shaku_actix::Inject;
use utoipa::ToSchema;
use uuid::Uuid;

// ? ---------------------------------------------------------------------------
// ? Configure application
// ? ---------------------------------------------------------------------------

pub fn configure(config: &mut web::ServiceConfig) {
    config.service(
        web::scope("/webhooks")
            .service(crate_webhook_url)
            .service(delete_webhook_url),
    );
}

// ? ---------------------------------------------------------------------------
// ? Define API structs
// ? ---------------------------------------------------------------------------

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateWebHookBody {
    pub url: String,
    pub action: WebHookDefaultAction,
}

// ? ---------------------------------------------------------------------------
// ? Define endpoints
// ? ---------------------------------------------------------------------------

#[utoipa::path(
    post,
    context_path = "/myc/managers/webhooks",
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
    delete,
    context_path = "/myc/managers/webhooks",
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
