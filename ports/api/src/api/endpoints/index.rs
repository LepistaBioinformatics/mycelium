use utoipa::OpenApi;

// ? ---------------------------------------------------------------------------
// ? Configure the API documentation
// ? ---------------------------------------------------------------------------

#[derive(OpenApi)]
#[openapi(
    paths(
        heath_check_endpoints::health_url,
        heath_check_endpoints::now_url,
    ),
    tags(
        (
            name = "health-check", 
            description = "Health check endpoints."
        )
    ),
)]
pub struct ApiDoc;

// ? ---------------------------------------------------------------------------
// ? Create endpoints module
// ? ---------------------------------------------------------------------------

pub mod heath_check_endpoints {

    use actix_web::{get, web, HttpResponse, Responder};
    use chrono::{Local, Utc};

    // ? -----------------------------------------------------------------------
    // ? Configure application
    // ? -----------------------------------------------------------------------

    pub fn configure(config: &mut web::ServiceConfig) {
        config.service(health_url).service(now_url);
    }

    // ? -----------------------------------------------------------------------
    // ? Define API paths
    // ? -----------------------------------------------------------------------

    /// Provide a health check endpoint.
    ///
    /// If the server is running it returns a 200 response with a JSON body
    /// containing the success message.
    #[utoipa::path(
        get,
        context_path = "/health",
        responses(
            (
                status = 200,
                description = "Health check passed.",
                body = String,
            ),
        ),
    )]
    #[get("/")]
    pub async fn health_url() -> impl Responder {
        HttpResponse::Ok().body("success".to_string())
    }

    /// Provide a datetime with the server's timezone.
    ///
    /// This is usual during system checks.
    #[utoipa::path(
        get,
        context_path = "/health",
        responses(
            (
                status = 200,
                description = "The current datetime with timezone.",
                body = String,
            ),
        ),
    )]
    #[get("/now")]
    pub async fn now_url() -> impl Responder {
        HttpResponse::Ok().body(Utc::now().with_timezone(&Local).to_string())
    }
}
