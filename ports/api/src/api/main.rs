extern crate myc_core;

mod config;
mod endpoints;
mod modules;
mod router;
mod settings;

use actix_cors::Cors;
use actix_web::{middleware::Logger, web, App, HttpServer};
use awc::Client;
use config::{configure as configure_injection_modules, SvcConfig};
use endpoints::{
    default_users::{
        account_endpoints as default_users_account_endpoints,
        profile_endpoints as default_users_profile_endpoints,
        ApiDoc as DefaultUsersApiDoc,
    },
    index::{heath_check_endpoints, ApiDoc as HealthCheckApiDoc},
    manager::{
        account_endpoints as manager_account_endpoints,
        error_code_endpoints as manager_error_code_endpoints,
        guest_endpoints as manager_guest_endpoints,
        guest_role_endpoints as manager_guest_role_endpoints,
        role_endpoints as manager_role_endpoints, ApiDoc as ManagerApiDoc,
    },
    staff::{
        account_endpoints as staff_account_endpoints, ApiDoc as StaffApiDoc,
    },
};
use log::{debug, info};
use myc_core::settings::init_in_memory_routes;
use myc_prisma::repositories::connector::generate_prisma_client_of_thread;
use reqwest::header::{
    ACCEPT, ACCESS_CONTROL_ALLOW_METHODS, ACCESS_CONTROL_ALLOW_ORIGIN,
    AUTHORIZATION, CONTENT_LENGTH, CONTENT_TYPE,
};
use router::route_request;
use settings::{GATEWAY_API_SCOPE, MYCELIUM_API_SCOPE};
use std::process::id as process_id;
use utoipa::OpenApi;
use utoipa_swagger_ui::{Config, SwaggerUi, Url};

// ? ---------------------------------------------------------------------------
// ? API fire elements
// ? ---------------------------------------------------------------------------

#[actix_web::main]
pub async fn main() -> std::io::Result<()> {
    info!("Initializing Logging configuration.");
    env_logger::init();

    info!("Initializing API configuration.");
    let config = SvcConfig::new();

    info!("Initializing routes.");
    init_in_memory_routes().await;

    info!("Start the database connectors.");
    generate_prisma_client_of_thread(process_id()).await;

    info!("Set the server configuration.");
    let server = HttpServer::new(move || {
        let config = SvcConfig::new();
        debug!("Configured Origins: {:?}", config.allowed_origins);

        let cors = Cors::default()
            //.allowed_origin_fn(move |origin, _| {
            //    origins.contains(&origin.to_str().unwrap_or("").to_string())
            //})
            .allow_any_origin()
            .send_wildcard()
            .disable_vary_header()
            .allowed_headers(vec![
                ACCESS_CONTROL_ALLOW_METHODS,
                ACCESS_CONTROL_ALLOW_ORIGIN,
                CONTENT_LENGTH,
                AUTHORIZATION,
                ACCEPT,
                CONTENT_TYPE,
            ])
            .allow_any_method()
            .max_age(3600);

        debug!("Configured Cors: {:?}", cors);

        App::new()
            // ? ---------------------------------------------------------------
            // ? Configure CORS policies
            // ? ---------------------------------------------------------------
            .wrap(cors)
            // ? ---------------------------------------------------------------
            // ? Configure Log elements
            // ? ---------------------------------------------------------------
            // These wrap create the basic log elements and exclude the health
            // check route.
            .wrap(Logger::default().exclude_regex("/health/*"))
            // These wrap create the agent that performed the request and
            // exclude the health check route.
            .wrap(Logger::new("%a %{User-Agent}i").exclude_regex("/health/*"))
            // ? ---------------------------------------------------------------
            // ? Configure Injection modules
            // ? ---------------------------------------------------------------
            .configure(configure_injection_modules)
            // ? ---------------------------------------------------------------
            // ? Configure mycelium routes
            // ? ---------------------------------------------------------------
            .service(
                web::scope(&format!("/{}", MYCELIUM_API_SCOPE))
                    //
                    // Index
                    //
                    .service(
                        web::scope("/health")
                            .configure(heath_check_endpoints::configure),
                    )
                    //
                    // Default Users
                    //
                    .service(
                        web::scope("/default-users")
                            .configure(
                                default_users_account_endpoints::configure,
                            )
                            .configure(
                                default_users_profile_endpoints::configure,
                            ),
                    )
                    //
                    // Manager
                    //
                    .service(
                        web::scope("/managers")
                            .configure(manager_account_endpoints::configure)
                            .configure(manager_error_code_endpoints::configure)
                            .configure(manager_guest_endpoints::configure)
                            .configure(manager_guest_role_endpoints::configure)
                            .configure(manager_role_endpoints::configure),
                    )
                    //
                    // Staff
                    //
                    .service(
                        web::scope("/staffs")
                            .configure(staff_account_endpoints::configure),
                    ),
            )
            // ? ---------------------------------------------------------------
            // ? Configure API documentation
            // ? ---------------------------------------------------------------
            .service(
                SwaggerUi::new("/swagger-ui/{_:.*}")
                    .config(
                        Config::default()
                            .filter(true)
                            .show_extensions(true)
                            .show_common_extensions(true)
                            .with_credentials(true)
                            .request_snippets_enabled(true)
                            .oauth2_redirect_url(config.oauth2_redirect_url),
                    )
                    .urls(vec![
                        (
                            Url::with_primary(
                                "System monitoring",
                                "/doc/monitoring-openapi.json",
                                true,
                            ),
                            HealthCheckApiDoc::openapi(),
                        ),
                        (
                            Url::new(
                                "Default Users Endpoints",
                                "/doc/default-users-openapi.json",
                            ),
                            DefaultUsersApiDoc::openapi(),
                        ),
                        (
                            Url::new(
                                "Manager Users Endpoints",
                                "/doc/manager-openapi.json",
                            ),
                            ManagerApiDoc::openapi(),
                        ),
                        (
                            Url::new(
                                "Staff Users Endpoints",
                                "/doc/staff-openapi.json",
                            ),
                            StaffApiDoc::openapi(),
                        ),
                    ]),
            )
            // ? ---------------------------------------------------------------
            // ? Configure gateway routes
            // ? ---------------------------------------------------------------
            .app_data(web::Data::new(Client::default()))
            .app_data(web::Data::new(config.gateway_timeout))
            .service(
                web::scope(&format!("/{}", GATEWAY_API_SCOPE))
                    .default_service(web::to(route_request)),
            )
    });

    info!("Fire the server.");
    server
        .bind((config.service_ip, config.service_port))?
        .workers(config.service_workers as usize)
        .run()
        .await
}
