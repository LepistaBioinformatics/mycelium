use crate::dtos::MyceliumProfileData;

use actix_web::{get, web, HttpResponse, Responder};
use myc_core::domain::dtos::profile::{LicensedResources, TenantsOwnership};
use myc_http_tools::{
    settings::MYCELIUM_AI_AWARE, utils::HttpJsonResponse, Profile,
};
use serde::Deserialize;
use utoipa::IntoParams;

// ? ---------------------------------------------------------------------------
// ? Configure application
// ? ---------------------------------------------------------------------------

pub fn configure(config: &mut web::ServiceConfig) {
    config.service(fetch_profile_url);
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

/// Fetch a user's profile.
#[utoipa::path(
    get,
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
    tag = MYCELIUM_AI_AWARE
)]
#[get("")]
pub async fn fetch_profile_url(
    query: web::Query<ProfileParams>,
    mut profile: MyceliumProfileData,
) -> impl Responder {
    match query.with_url.unwrap_or(true) {
        true => {
            //
            // Try to set the licensed resources as a string list
            //
            if let Some(licensed_resources) = profile.licensed_resources {
                let resources = match licensed_resources {
                    LicensedResources::Urls(urls) => urls,
                    LicensedResources::Records(records) => records
                        .iter()
                        .map(|r| r.to_string())
                        .collect::<Vec<String>>(),
                };

                profile.licensed_resources = match resources.is_empty() {
                    true => None,
                    false => Some(LicensedResources::Urls(resources)),
                }
            };

            //
            // Try to set the tenant ownership as a string list
            //
            if let Some(tenants_ownership) = profile.tenants_ownership {
                let ownerships = match tenants_ownership {
                    TenantsOwnership::Urls(urls) => urls,
                    TenantsOwnership::Records(records) => records
                        .iter()
                        .map(|r| r.to_string())
                        .collect::<Vec<String>>(),
                };

                profile.tenants_ownership = match ownerships.is_empty() {
                    true => None,
                    false => Some(TenantsOwnership::Urls(ownerships)),
                };
            };

            HttpResponse::Ok().json(profile.to_profile())
        }
        _ => HttpResponse::Ok().json(profile.to_profile()),
    }
}
