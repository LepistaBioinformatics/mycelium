use actix_web::{get, web, HttpResponse, Responder};
use myc_http_tools::middleware::MyceliumProfileData;

// ? ---------------------------------------------------------------------------
// ? Configure application
// ? ---------------------------------------------------------------------------

pub fn configure(config: &mut web::ServiceConfig) {
    config.service(web::scope("/profiles").service(fetch_profile));
}

// ? ---------------------------------------------------------------------------
// ? Define API paths
//
// Account
//
// ? ---------------------------------------------------------------------------

#[utoipa::path(
        get,
        context_path = "/myc/default-users/profiles",
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
