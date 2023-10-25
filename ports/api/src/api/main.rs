mod config;
mod endpoints;
mod models;
mod modules;
mod router;
mod settings;

use actix_cors::Cors;
use actix_web::{middleware::Logger, web, App, HttpServer};
use awc::Client;
use config::injectors::configure as configure_injection_modules;
use endpoints::{
    default_users::{
        account_endpoints as default_users_account_endpoints,
        profile_endpoints as default_users_profile_endpoints,
        user_endpoints as default_users_user_endpoints,
        ApiDoc as DefaultUsersApiDoc,
    },
    index::{heath_check_endpoints, ApiDoc as HealthCheckApiDoc},
    manager::{
        account_endpoints as manager_account_endpoints,
        error_code_endpoints as manager_error_code_endpoints,
        guest_endpoints as manager_guest_endpoints,
        guest_role_endpoints as manager_guest_role_endpoints,
        role_endpoints as manager_role_endpoints,
        webhook_endpoints as manager_webhook_endpoints,
        ApiDoc as ManagerApiDoc,
    },
    staff::{
        account_endpoints as staff_account_endpoints, ApiDoc as StaffApiDoc,
    },
};
use log::{debug, info};
use models::config_handler::ConfigHandler;
use myc_config::optional_config::OptionalConfig;
use myc_core::settings::init_in_memory_routes;
use myc_http_tools::providers::google_handlers;
use myc_prisma::repositories::connector::generate_prisma_client_of_thread;
use myc_smtp::settings::init_smtp_config_from_file;
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};
use reqwest::header::{
    ACCEPT, ACCESS_CONTROL_ALLOW_CREDENTIALS, ACCESS_CONTROL_ALLOW_METHODS,
    ACCESS_CONTROL_ALLOW_ORIGIN, AUTHORIZATION, CONTENT_LENGTH, CONTENT_TYPE,
};
use router::route_request;
use settings::{GATEWAY_API_SCOPE, MYCELIUM_API_SCOPE};
use std::{path::PathBuf, process::id as process_id};
use utoipa::OpenApi;
use utoipa_swagger_ui::{Config, SwaggerUi, Url};

// ? ---------------------------------------------------------------------------
// ? API fire elements
// ? ---------------------------------------------------------------------------

#[actix_web::main]
pub async fn main() -> std::io::Result<()> {
    // ? -----------------------------------------------------------------------
    // ? Initialize services configuration
    //
    // All configurations for the core, ports, and adapters layers should exists
    // into the configuration file. Such file are loaded here.
    //
    // ? -----------------------------------------------------------------------
    info!("Initializing services configuration");

    let env_config_path = match std::env::var("SETTINGS_PATH") {
        Ok(path) => path,
        Err(err) => panic!("Error on get env `SETTINGS_PATH`: {err}"),
    };

    let config =
        match ConfigHandler::init_from_file(PathBuf::from(env_config_path)) {
            Ok(res) => res,
            Err(err) => panic!("Error on init config: {err}"),
        };

    let api_config = config.api.clone();

    // ? -----------------------------------------------------------------------
    // ? Configure logging level
    // ? -----------------------------------------------------------------------
    info!("Initializing Logging configuration");
    env_logger::init_from_env(
        env_logger::Env::new()
            .default_filter_or(api_config.to_owned().logging_level),
    );

    // ? -----------------------------------------------------------------------
    // ? Routes should be used on API gateway
    //
    // When users perform queries to the API gateway, the gateway should
    // redirect the request to the correct service. Services are loaded into
    // memory and the gateway should know the routes during their execution.
    //
    // ? -----------------------------------------------------------------------
    info!("Initializing routes");
    init_in_memory_routes(Some(config.api.routes.clone())).await;

    // ? -----------------------------------------------------------------------
    // ? Routes should be used on API gateway
    //
    // When users perform queries to the API gateway, the gateway should
    // redirect the request to the correct service. Services are loaded into
    // memory and the gateway should know the routes during their execution.
    //
    // ? -----------------------------------------------------------------------
    info!("Initializing SMTP configs");
    init_smtp_config_from_file(None, Some(config.smtp)).await;

    // ? -----------------------------------------------------------------------
    // ? Here the current thread receives an instance of the prisma client.
    //
    // Each thread should contains a prisma instance. Otherwise the application
    // should raise an adapter error on try to perform the first database query.
    //
    // ? -----------------------------------------------------------------------
    info!("Start the database connectors");

    std::env::set_var("DATABASE_URL", config.prisma.database_url.clone());
    generate_prisma_client_of_thread(process_id()).await;

    // ? -----------------------------------------------------------------------
    // ? Configure the server
    // ? -----------------------------------------------------------------------
    info!("Set the server configuration");
    let server = HttpServer::new(move || {
        let api_config = config.api.clone();
        let auth_config = config.auth.clone();
        let token_config = config.core.token.clone();

        let cors = Cors::default()
            .allowed_origin_fn(move |origin, _| {
                api_config
                    .allowed_origins
                    .contains(&origin.to_str().unwrap_or("").to_string())
            })
            .allowed_headers(vec![
                ACCESS_CONTROL_ALLOW_CREDENTIALS,
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

        // ? -------------------------------------------------------------------
        // ? Configure base application
        // ? -------------------------------------------------------------------

        let app = App::new()
            .app_data(web::Data::new(token_config).clone())
            .app_data(web::Data::new(config.auth.clone()).clone());

        // ? -------------------------------------------------------------------
        // ? Configure base mycelium scope
        // ? -------------------------------------------------------------------
        let mycelium_scope = web::scope(&format!("/{}", MYCELIUM_API_SCOPE))
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
                    .configure(default_users_account_endpoints::configure)
                    .configure(default_users_profile_endpoints::configure)
                    .configure(default_users_user_endpoints::configure),
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
                    .configure(manager_role_endpoints::configure)
                    .configure(manager_webhook_endpoints::configure),
            )
            //
            // Staff
            //
            .service(
                web::scope("/staffs")
                    .configure(staff_account_endpoints::configure),
            );

        // ? -------------------------------------------------------------------
        // ? Configure authentication elements
        //
        // Google OAuth2
        //
        // ? -------------------------------------------------------------------
        let gateway_scopes = match auth_config.google {
            OptionalConfig::Enabled(_) => {
                debug!("Configure Google authentication");
                //
                // Configure OAuth2 Scope
                //
                mycelium_scope.service(
                    web::scope("/auth/google")
                        .configure(google_handlers::configure),
                )
            }
            _ => mycelium_scope,
        };

        app
            // ? ---------------------------------------------------------------
            // ? Configure CORS policies
            // ? ---------------------------------------------------------------
            .wrap(cors)
            // ? ---------------------------------------------------------------
            // ? Configure Log elements
            // ? ---------------------------------------------------------------
            // These wrap create the basic log elements and exclude the health
            // check route.
            .wrap(
                Logger::new("%a %{User-Agent}i")
                    .exclude_regex("/health/*")
                    .exclude_regex("/swagger-ui/*")
                    .exclude_regex("/auth/google/*")
                    .exclude_regex("/auth/azure/*"),
            )
            // ? ---------------------------------------------------------------
            // ? Configure Injection modules
            // ? ---------------------------------------------------------------
            .configure(configure_injection_modules)
            // ? ---------------------------------------------------------------
            // ? Configure mycelium routes
            // ? ---------------------------------------------------------------
            .service(gateway_scopes)
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
                            .request_snippets_enabled(true),
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
            .app_data(web::Data::new(api_config.gateway_timeout))
            .service(
                web::scope(&format!("/{}", GATEWAY_API_SCOPE))
                    .default_service(web::to(route_request)),
            )
    });

    let address = (
        api_config.to_owned().service_ip,
        api_config.to_owned().service_port,
    );

    info!("Listening on Address and Port: {:?}: ", address);

    if let OptionalConfig::Enabled(tls_config) = api_config.to_owned().tls {
        let api_config = api_config.clone();

        info!("Load TLS cert and key");

        // To create a self-signed temporary cert for testing:
        //
        // openssl req
        //     -x509 \
        //     -newkey rsa:4096 \
        //     -nodes \
        //     -keyout key.pem \
        //     -out cert.pem \
        //     -days 365 \
        //     -subj '/CN=localhost'
        //
        let mut builder =
            SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();

        builder
            .set_private_key_file(
                tls_config.tls_key_path.unwrap(),
                SslFiletype::PEM,
            )
            .unwrap();

        builder
            .set_certificate_chain_file(tls_config.tls_cert_path.unwrap())
            .unwrap();

        info!("Fire the server with TLS");
        return server
            .bind_openssl(format!("{}:{}", address.0, address.1), builder)?
            .workers(api_config.service_workers.to_owned() as usize)
            .run()
            .await;
    }

    info!("Fire the server without TLS");
    server
        .bind(address)?
        .workers(api_config.service_workers as usize)
        .run()
        .await
}
