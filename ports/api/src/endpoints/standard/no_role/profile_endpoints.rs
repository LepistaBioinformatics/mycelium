use crate::{
    dtos::MyceliumProfileData,
    endpoints::shared::{build_actor_context, UrlGroup},
};

use actix_web::{get, web, HttpResponse, Responder};
use myc_core::domain::{actors::ActorName, dtos::profile::LicensedResources};
use serde::Deserialize;
use utoipa::IntoParams;

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

#[derive(Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct ProfileParams {
    with_url: Option<bool>,
}

#[utoipa::path(
    get,
    context_path = build_actor_context(ActorName::NoRole, UrlGroup::Profile),
    params(ProfileParams),
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
pub async fn fetch_profile(
    query: web::Query<ProfileParams>,
    mut profile: MyceliumProfileData,
) -> impl Responder {
    match query.with_url.unwrap_or(true) {
        true => {
            if let Some(licensed_resources) = profile.licensed_resources {
                let resources = match licensed_resources {
                    LicensedResources::Records(records) => {
                        records.iter().map(|r| r.to_string()).collect()
                    }
                    LicensedResources::Urls(urls) => urls,
                };

                profile.licensed_resources =
                    Some(LicensedResources::Urls(resources));

                HttpResponse::Ok().json(profile.to_profile())
            } else {
                HttpResponse::NoContent().finish()
            }
        }
        _ => HttpResponse::Ok().json(profile.to_profile()),
    }
}
