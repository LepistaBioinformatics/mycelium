use crate::endpoints::shared::UrlScope;

use actix_web::{get, web, HttpResponse, Responder};
use myc_core::domain::actors::DefaultActor;

// ? ---------------------------------------------------------------------------
// ? Configure application
// ? ---------------------------------------------------------------------------

pub fn configure(config: &mut web::ServiceConfig) {
    config.service(web::scope("/actors").service(list_actors_url));
}

// ? ---------------------------------------------------------------------------
// ? Define API paths
// ? ---------------------------------------------------------------------------

/// Provide a datetime with the server's timezone.
///
/// This is usual during system checks.
#[utoipa::path(
        get,
        context_path = &format!(
            "{}/aux/actors", UrlScope::Standards.build_myc_path(),
        ),
        responses(
            (
                status = 200,
                description = "The current datetime with timezone.",
                body = String,
            ),
        ),
    )]
#[get("/")]
pub async fn list_actors_url() -> impl Responder {
    HttpResponse::Ok().json(
        vec![
            DefaultActor::NoRole,
            DefaultActor::SubscriptionAccountManager,
            DefaultActor::UserAccountManager,
            DefaultActor::GuestManager,
            DefaultActor::SystemManager,
        ]
        .into_iter()
        .map(|x| x.to_string())
        .collect::<Vec<String>>(),
    )
}
