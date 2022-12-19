extern crate myc;
mod api_config;
mod endpoints;
mod modules;

use actix_cors::Cors;
use actix_web::middleware::Logger;
use actix_web::{App, HttpServer};
use api_config::{InjectableModulesConfig, SvcConfig};
use endpoints::{
    index::{heath_check_endpoints, ApiDoc as HealthCheckApiDoc},
    service::{service_endpoints, ApiDoc as ServiceApiDoc},
};
use log::info;
use myc::adapters::repositories::sql_db::connector::generate_prisma_client_of_thread;
use reqwest::header::{
    ACCEPT, ACCESS_CONTROL_ALLOW_METHODS, ACCESS_CONTROL_ALLOW_ORIGIN,
    AUTHORIZATION, CONTENT_LENGTH, CONTENT_TYPE,
};
use std::process::id as process_id;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

// ? ---------------------------------------------------------------------------
// ? API fire elements
// ? ---------------------------------------------------------------------------

#[actix_web::main]
pub async fn main() -> std::io::Result<()> {
    env_logger::init();

    info!("Initialize configuration.");
    let config = SvcConfig::new();

    info!("Start the prisma engine connectors.");
    let mods = InjectableModulesConfig::new().await;

    info!("Start the database connectors.");
    generate_prisma_client_of_thread(process_id()).await;

    info!("Set the server configuration.");
    let server = HttpServer::new(move || {
        let origins = SvcConfig::new().allowed_origins;

        let cors = Cors::default()
            .allowed_origin_fn(move |origin, _| {
                origins.contains(&origin.to_str().unwrap_or("").to_string())
            })
            .allow_any_origin()
            .allowed_headers(vec![
                ACCESS_CONTROL_ALLOW_METHODS,
                ACCESS_CONTROL_ALLOW_ORIGIN,
                CONTENT_LENGTH,
                AUTHORIZATION,
                ACCEPT,
            ])
            .allowed_header(CONTENT_TYPE)
            .allowed_methods(vec![
                "POST", "PUT", "PATCH", "GET", "OPTIONS", "HEAD", "DELETE",
            ])
            .max_age(3600);

        App::new()
            // ? ---------------------------------------------------------------
            // ? Configure CORS policies
            // ? ---------------------------------------------------------------
            .wrap(cors)
            // ? ---------------------------------------------------------------
            // ? Configure Log elements
            // ? ---------------------------------------------------------------
            .wrap(Logger::default())
            .wrap(Logger::new("%a %{User-Agent}i"))
            // ? ---------------------------------------------------------------
            // ? Configure OpenApi definitions
            // ? ---------------------------------------------------------------
            .configure(heath_check_endpoints::configure)
            .configure(service_endpoints::configure)
            // ? ---------------------------------------------------------------
            // ? Configure Injection modules
            // ? ---------------------------------------------------------------
            .app_data(mods.profile_fetching_module.clone())
            .service(
                SwaggerUi::new("/swagger-ui/{_:.*}")
                    .url(
                        "/doc/monitoring/openapi.json",
                        HealthCheckApiDoc::openapi(),
                    )
                    .url("/doc/service/openapi.json", ServiceApiDoc::openapi()),
            )
    });

    info!("Fire the server.");
    server
        .bind((config.service_ip, config.service_port))?
        .run()
        .await
}
