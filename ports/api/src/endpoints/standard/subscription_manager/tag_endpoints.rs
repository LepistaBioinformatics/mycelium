use crate::{
    dtos::MyceliumProfileData,
    endpoints::{shared::UrlGroup, standard::shared::build_actor_context},
    modules::{
        AccountTagDeletionModule, AccountTagRegistrationModule,
        AccountTagUpdatingModule,
    },
};

use actix_web::{delete, post, put, web, HttpResponse, Responder};
use myc_core::{
    domain::{
        actors::ActorName,
        dtos::tag::Tag,
        entities::{
            AccountTagDeletion, AccountTagRegistration, AccountTagUpdating,
        },
    },
    use_cases::roles::standard::subscription_manager::tag::{
        delete_tag, register_tag, update_tag,
    },
};
use myc_http_tools::utils::JsonError;
use mycelium_base::entities::{
    DeletionResponseKind, GetOrCreateResponseKind, UpdatingResponseKind,
};
use serde::Deserialize;
use shaku_actix::Inject;
use std::collections::HashMap;
use utoipa::ToSchema;
use uuid::Uuid;

// ? ---------------------------------------------------------------------------
// ? Configure application
// ? ---------------------------------------------------------------------------

pub fn configure(config: &mut web::ServiceConfig) {
    config
        .service(register_tag_url)
        .service(update_tag_url)
        .service(delete_tag_url);
}

// ? ---------------------------------------------------------------------------
// ? Define API structs
// ? ---------------------------------------------------------------------------

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateTagBody {
    value: String,
    meta: HashMap<String, String>,
}

// ? ---------------------------------------------------------------------------
// ? Define API paths
//
// Account
//
// ? ---------------------------------------------------------------------------

#[utoipa::path(
    post,
    context_path = build_actor_context(ActorName::SubscriptionManager, UrlGroup::Accounts),
    params(
        ("id" = Uuid, Path, description = "The account primary key."),
    ),
    request_body = CreateTagBody,
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
            status = 400,
            description = "Bad request.",
            body = JsonError,
        ),
        (
            status = 201,
            description = "Tag successfully registered.",
            body = AnalysisTag,
        ),
    ),
)]
#[post("/{id}/tags/")]
pub async fn register_tag_url(
    profile: MyceliumProfileData,
    path: web::Path<(Uuid,)>,
    body: web::Json<CreateTagBody>,
    tag_registration_repo: Inject<
        AccountTagRegistrationModule,
        dyn AccountTagRegistration,
    >,
) -> impl Responder {
    match register_tag(
        profile.to_profile(),
        body.value.to_owned(),
        body.meta.to_owned(),
        path.into_inner().0,
        Box::from(&*tag_registration_repo),
    )
    .await
    {
        Err(err) => HttpResponse::InternalServerError()
            .json(JsonError::new(err.to_string())),
        Ok(res) => match res {
            GetOrCreateResponseKind::Created(record) => {
                HttpResponse::Created().json(record)
            }
            GetOrCreateResponseKind::NotCreated(_, msg) => {
                HttpResponse::BadRequest().json(JsonError::new(msg))
            }
        },
    }
}

#[utoipa::path(
    put,
    context_path = build_actor_context(ActorName::SubscriptionManager, UrlGroup::Accounts),
    params(
        ("id" = Uuid, Path, description = "The account primary key."),
        ("tag_id" = Uuid, Path, description = "The tag primary key."),
    ),
    request_body = CreateTagBody,
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
            status = 400,
            description = "Bad request.",
            body = JsonError,
        ),
        (
            status = 201,
            description = "Tag successfully registered.",
            body = AnalysisTag,
        ),
    ),
)]
#[put("/{id}/tags/{tag_id}")]
pub async fn update_tag_url(
    profile: MyceliumProfileData,
    path: web::Path<(Uuid, Uuid)>,
    body: web::Json<CreateTagBody>,
    tag_updating_repo: Inject<AccountTagUpdatingModule, dyn AccountTagUpdating>,
) -> impl Responder {
    match update_tag(
        profile.to_profile(),
        Tag {
            id: path.into_inner().1,
            value: body.value.to_owned(),
            meta: Some(body.meta.to_owned()),
        },
        Box::from(&*tag_updating_repo),
    )
    .await
    {
        Err(err) => HttpResponse::InternalServerError()
            .json(JsonError::new(err.to_string())),
        Ok(res) => match res {
            UpdatingResponseKind::Updated(record) => {
                HttpResponse::Accepted().json(record)
            }
            UpdatingResponseKind::NotUpdated(_, msg) => {
                HttpResponse::BadRequest().json(JsonError::new(msg))
            }
        },
    }
}

#[utoipa::path(
    delete,
    context_path = build_actor_context(ActorName::SubscriptionManager, UrlGroup::Accounts),
    params(
        ("id" = Uuid, Path, description = "The account primary key."),
        ("tag_id" = Uuid, Path, description = "The tag primary key."),
    ),
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
            status = 400,
            description = "Bad request.",
            body = JsonError,
        ),
        (
            status = 201,
            description = "Tag successfully registered.",
            body = AnalysisTag,
        ),
    ),
)]
#[delete("/{id}/tags/{tag_id}")]
pub async fn delete_tag_url(
    profile: MyceliumProfileData,
    path: web::Path<(Uuid, Uuid)>,
    tag_deletion_repo: Inject<AccountTagDeletionModule, dyn AccountTagDeletion>,
) -> impl Responder {
    match delete_tag(
        profile.to_profile(),
        path.into_inner().1,
        Box::from(&*tag_deletion_repo),
    )
    .await
    {
        Err(err) => HttpResponse::InternalServerError()
            .json(JsonError::new(err.to_string())),
        Ok(res) => match res {
            DeletionResponseKind::Deleted => HttpResponse::NoContent().finish(),
            DeletionResponseKind::NotDeleted(_, msg) => {
                HttpResponse::BadRequest().json(JsonError::new(msg))
            }
        },
    }
}
