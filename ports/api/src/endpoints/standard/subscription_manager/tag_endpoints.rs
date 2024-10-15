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
use myc_http_tools::{
    utils::HttpJsonResponse,
    wrappers::default_response_to_http_response::{
        delete_response_kind, get_or_create_response_kind,
        updating_response_kind,
    },
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
    context_path = build_actor_context(ActorName::SubscriptionManager, UrlGroup::Tags),
    params(
        ("id" = Uuid, Path, description = "The account primary key."),
    ),
    request_body = CreateTagBody,
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
        Ok(res) => get_or_create_response_kind(res),
        Err(err) => HttpResponse::InternalServerError()
            .json(HttpJsonResponse::new_message(err.to_string())),
    }
}

#[utoipa::path(
    put,
    context_path = build_actor_context(ActorName::SubscriptionManager, UrlGroup::Tags),
    params(
        ("id" = Uuid, Path, description = "The account primary key."),
        ("tag_id" = Uuid, Path, description = "The tag primary key."),
    ),
    request_body = CreateTagBody,
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
        Ok(res) => updating_response_kind(res),
        Err(err) => HttpResponse::InternalServerError()
            .json(HttpJsonResponse::new_message(err.to_string())),
    }
}

#[utoipa::path(
    delete,
    context_path = build_actor_context(ActorName::SubscriptionManager, UrlGroup::Tags),
    params(
        ("id" = Uuid, Path, description = "The account primary key."),
        ("tag_id" = Uuid, Path, description = "The tag primary key."),
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
        Ok(res) => delete_response_kind(res),
        Err(err) => HttpResponse::InternalServerError()
            .json(HttpJsonResponse::new_message(err.to_string())),
    }
}
