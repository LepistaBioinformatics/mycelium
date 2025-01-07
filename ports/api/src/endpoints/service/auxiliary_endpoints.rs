use actix_web::{get, web, HttpResponse, Responder};
use myc_http_tools::SystemActor;

// ? ---------------------------------------------------------------------------
// ? Configure application
// ? ---------------------------------------------------------------------------

pub fn configure(config: &mut web::ServiceConfig) {
    config.service(web::scope("/auxiliary").service(list_actors_url));
}

// ? ---------------------------------------------------------------------------
// ? Define API paths
// ? ---------------------------------------------------------------------------

/// Provide a datetime with the server's timezone.
///
/// This is usual during system checks.
#[utoipa::path(
    get,
    responses(
        (
            status = 200,
            description = "The current datetime with timezone.",
            body = String,
        ),
    ),
    security(())
)]
#[get("/actors")]
pub async fn list_actors_url() -> impl Responder {
    HttpResponse::Ok().json(
        vec![
            SystemActor::CustomRole("CustomRole".to_string()),
            SystemActor::Beginner,
            SystemActor::SubscriptionsManager,
            SystemActor::UsersManager,
            SystemActor::AccountManager,
            SystemActor::GuestsManager,
            SystemActor::SystemManager,
            SystemActor::TenantOwner,
            SystemActor::TenantManager,
        ]
        .into_iter()
        .map(|x| x.to_string())
        .collect::<Vec<String>>(),
    )
}
