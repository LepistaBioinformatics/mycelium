pub mod heath_check_endpoints;

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
