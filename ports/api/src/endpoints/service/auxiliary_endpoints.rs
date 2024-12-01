use crate::endpoints::shared::{build_actor_context, UrlGroup};

use actix_web::{get, web, HttpResponse, Responder};
use myc_http_tools::ActorName;

// ? ---------------------------------------------------------------------------
// ? Configure application
// ? ---------------------------------------------------------------------------

pub fn configure(config: &mut web::ServiceConfig) {
    config.service(
        web::scope("/auxiliary")
            .service(list_actors_url)
            .service(list_role_controlled_main_routes_url),
    );
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
)]
#[get("/actors")]
pub async fn list_actors_url() -> impl Responder {
    HttpResponse::Ok().json(
        vec![
            ActorName::CustomRole("CustomRole".to_string()),
            ActorName::Beginner,
            ActorName::SubscriptionsManager,
            ActorName::UsersManager,
            ActorName::AccountManager,
            ActorName::GuestManager,
            ActorName::SystemManager,
            ActorName::TenantOwner,
            ActorName::TenantManager,
        ]
        .into_iter()
        .map(|x| x.to_string())
        .collect::<Vec<String>>(),
    )
}

#[utoipa::path(
    get,
    responses(
        (
            status = 200,
            description = "The current datetime with timezone.",
            body = String,
        ),
    ),
)]
#[get("/role-controlled-routes")]
pub async fn list_role_controlled_main_routes_url() -> impl Responder {
    HttpResponse::Ok().json(
        [
            ActorName::Beginner,
            ActorName::SubscriptionsManager,
            ActorName::UsersManager,
            ActorName::TenantOwner,
            ActorName::TenantManager,
            ActorName::AccountManager,
            ActorName::GuestManager,
            ActorName::SystemManager,
        ]
        .into_iter()
        .flat_map(|actor| {
            [
                UrlGroup::Accounts,
                UrlGroup::ErrorCodes,
                UrlGroup::GuestRoles,
                UrlGroup::Guests,
                UrlGroup::Meta,
                UrlGroup::Owners,
                UrlGroup::Profile,
                UrlGroup::Roles,
                UrlGroup::Tags,
                UrlGroup::Tenants,
                UrlGroup::Tokens,
                UrlGroup::Users,
                UrlGroup::Webhooks,
            ]
            .into_iter()
            .map(|group| build_actor_context(actor.to_owned(), group))
            .collect::<Vec<String>>()
        })
        .into_iter()
        .collect::<Vec<String>>(),
    )
}
