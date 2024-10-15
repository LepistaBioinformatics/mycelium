use crate::{
    dtos::MyceliumProfileData,
    endpoints::{shared::UrlGroup, standard::shared::build_actor_context},
};

use actix_web::{get, web, HttpResponse, Responder};
use myc_core::domain::actors::ActorName;

// ? ---------------------------------------------------------------------------
// ? Configure application
// ? ---------------------------------------------------------------------------

pub fn configure(config: &mut web::ServiceConfig) {
    config.service(fetch_profile);
}

// ? ---------------------------------------------------------------------------
// ? Define API paths
//
// Account
//
// ? ---------------------------------------------------------------------------

#[utoipa::path(
    get,
    context_path = build_actor_context(ActorName::NoRole, UrlGroup::Profile),
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
            status = 204,
            description = "Not found.",
        ),
        (
            status = 200,
            description = "Profile fetching done.",
            body = Profile,
        ),
    ),
)]
#[get("/")]
pub async fn fetch_profile(profile: MyceliumProfileData) -> impl Responder {
    HttpResponse::Ok().json(profile.to_profile())
}
