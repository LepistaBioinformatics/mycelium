mod config;
mod dtos;
mod endpoints;
mod middleware;
mod models;
mod modules;
mod otel;
mod router;
mod settings;

use actix_cors::Cors;
use actix_session::{
    config::{BrowserSession, CookieContentSecurity},
    storage::CookieSessionStore,
    SessionMiddleware,
};
use actix_web::{
    cookie::{Key, SameSite},
    middleware::Logger,
    web, App, HttpServer,
};
use actix_web_opentelemetry::RequestTracing;
use awc::Client;
use config::injectors::configure as configure_injection_modules;
use endpoints::{
    index::{heath_check_endpoints, ApiDoc as HealthCheckApiDoc},
    manager::{tenant_endpoints, ApiDoc as ManagerApiDoc},
    staff::{
        account_endpoints as staff_account_endpoints, ApiDoc as StaffApiDoc,
    },
    standard::{
        configure as configure_standard_endpoints,
        ApiDoc as StandardUsersApiDoc,
    },
};
use models::{
    api_config::{LogFormat, LoggingTarget},
    config_handler::ConfigHandler,
};
use myc_config::optional_config::OptionalConfig;
use myc_core::{domain::dtos::http::Protocol, settings::init_in_memory_routes};
use myc_http_tools::providers::google_handlers;
use myc_prisma::repositories::connector::generate_prisma_client_of_thread;
use myc_smtp::settings::init_smtp_config_from_file;
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};
use opentelemetry::trace::TracerProvider;
use opentelemetry_otlp::WithExportConfig;
use otel::{metadata_from_headers, parse_otlp_headers_from_env};
use reqwest::header::{
    ACCEPT, ACCESS_CONTROL_ALLOW_CREDENTIALS, ACCESS_CONTROL_ALLOW_METHODS,
    ACCESS_CONTROL_ALLOW_ORIGIN, CONTENT_LENGTH, CONTENT_TYPE,
};
use router::route_request;
use settings::{GATEWAY_API_SCOPE, MYCELIUM_API_SCOPE};
use std::{path::PathBuf, process::id as process_id, str::FromStr};
use tracing::{debug, info};
use tracing_actix_web::TracingLogger;
use tracing_subscriber::prelude::*;
use tracing_subscriber::{fmt, EnvFilter};
use utoipa::OpenApi;
use utoipa_swagger_ui::{Config, SwaggerUi, Url};

fn session_middleware() -> SessionMiddleware<CookieSessionStore> {
    SessionMiddleware::builder(
        CookieSessionStore::default(),
        Key::from(&[0; 64]),
    )
    .cookie_name(String::from("myc-gw-cookie")) // arbitrary name
    .cookie_secure(true) // https only
    .session_lifecycle(BrowserSession::default()) // expire at end of session
    .cookie_same_site(SameSite::Lax)
    .cookie_content_security(CookieContentSecurity::Private) // encrypt
    .cookie_http_only(true) // disallow scripts from reading
    .build()
}

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
    // ? Configure logging and telemetry
    //
    // The logging and telemetry configuration should be initialized before the
    // server starts. The configuration should be set to the server and the
    // server should be started.
    //
    // ? -----------------------------------------------------------------------
    info!("Initializing Logging and Telemetry configuration");

    let logging_config = api_config.to_owned().logging;

    let (non_blocking, _guard) = match logging_config.target.to_owned() {
        //
        // If a log file is provided, log to the file
        //
        Some(LoggingTarget::File { path }) => {
            let mut log_file = PathBuf::from(path);

            let binding = log_file.to_owned();
            let parent_dir = binding
                .parent()
                .expect("Log file parent directory not found");

            match logging_config.format {
                LogFormat::Jsonl => {
                    log_file.set_extension("jsonl");
                }
                LogFormat::Ansi => {
                    log_file.set_extension("log");
                }
            };

            if log_file.exists() {
                std::fs::remove_file(&log_file)?;
            }

            let file_name =
                log_file.file_name().expect("Log file name not found");

            let file_appender =
                tracing_appender::rolling::never(parent_dir, file_name);

            tracing_appender::non_blocking(file_appender)
        }
        //
        // If no log file is provided, log to stderr
        //
        _ => tracing_appender::non_blocking(std::io::stderr()),
    };

    if let Some(LoggingTarget::Jaeger {
        name,
        protocol,
        host,
        port,
    }) = logging_config.target
    {
        //
        // Jaeger logging configurations
        //

        std::env::set_var("OTEL_SERVICE_NAME", name.to_owned());
        let headers = parse_otlp_headers_from_env();
        let tracer = opentelemetry_otlp::new_pipeline().tracing();

        let tracer = (match protocol {
            Protocol::Grpc => {
                let exporter = opentelemetry_otlp::new_exporter()
                    .tonic()
                    .with_endpoint(format!("{}://{}:{}", protocol, host, port))
                    .with_metadata(metadata_from_headers(headers));

                tracer.with_exporter(exporter)
            }
            _ => {
                let exporter = opentelemetry_otlp::new_exporter()
                    .http()
                    .with_endpoint(format!(
                        "{}://{}:{}/v1/logs",
                        protocol, host, port
                    ))
                    .with_headers(headers.into_iter().collect());

                tracer.with_exporter(exporter)
            }
        })
        .install_batch(opentelemetry_sdk::runtime::Tokio)
        .expect("Failed to install OpenTelemetry tracer")
        .tracer(name);

        let telemetry_layer =
            tracing_opentelemetry::layer().with_tracer(tracer);

        tracing_subscriber::Registry::default()
            .with(telemetry_layer)
            .init();
    } else {
        //
        // Default logging configurations
        //

        let tracing_formatting_layer = tracing_subscriber::fmt()
            .event_format(
                fmt::format()
                    .with_level(true)
                    .with_target(false)
                    .with_thread_ids(true)
                    .with_file(false)
                    .with_line_number(false),
            )
            .with_writer(non_blocking)
            .with_env_filter(
                EnvFilter::from_str(logging_config.level.as_str()).unwrap(),
            );

        match logging_config.format {
            LogFormat::Ansi => tracing_formatting_layer.pretty().init(),
            LogFormat::Jsonl => tracing_formatting_layer.json().init(),
        };
    };

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

    std::env::set_var(
        "DATABASE_URL",
        match config.prisma.database_url.get_or_error() {
            Ok(url) => url,
            Err(err) => panic!("Error on get database url: {err}"),
        },
    );

    generate_prisma_client_of_thread(process_id()).await;

    // ? -----------------------------------------------------------------------
    // ? Configure the server
    // ? -----------------------------------------------------------------------
    info!("Set the server configuration");
    let server = HttpServer::new(move || {
        let api_config = config.api.clone();
        let auth_config = config.auth.clone();
        let token_config = config.core.account_life_cycle.clone();

        let cors = Cors::default()
            .allowed_origin_fn(move |origin, _| {
                api_config
                    .allowed_origins
                    .contains(&origin.to_str().unwrap_or("").to_string())
            })
            .expose_headers(vec![
                ACCESS_CONTROL_ALLOW_CREDENTIALS,
                ACCESS_CONTROL_ALLOW_METHODS,
                ACCESS_CONTROL_ALLOW_ORIGIN,
                CONTENT_LENGTH,
                CONTENT_TYPE,
                ACCEPT,
            ])
            .allow_any_header()
            .allow_any_method()
            .max_age(3600);

        debug!("Configured Cors: {:?}", cors);

        // ? -------------------------------------------------------------------
        // ? Configure base application
        // ? -------------------------------------------------------------------

        let app = App::new()
            .wrap(RequestTracing::new())
            .wrap(TracingLogger::default())
            .app_data(web::Data::new(token_config).clone())
            .app_data(web::Data::new(auth_config.to_owned()).clone());

        // ? -------------------------------------------------------------------
        // ? Configure base mycelium scope
        // ? -------------------------------------------------------------------
        let mycelium_scope = web::scope(&format!("/{}", MYCELIUM_API_SCOPE))
            //
            // Index
            //
            .service(
                web::scope(
                    format!("/{}", endpoints::shared::UrlScope::Health)
                        .as_str(),
                )
                .configure(heath_check_endpoints::configure),
            )
            //
            // Manager Users
            //
            .service(
                web::scope(
                    format!("/{}", endpoints::shared::UrlScope::Managers)
                        .as_str(),
                )
                .configure(tenant_endpoints::configure),
            )
            //
            // Standard Users
            //
            .service(
                web::scope(
                    format!("/{}", endpoints::shared::UrlScope::Standards)
                        .as_str(),
                )
                .configure(configure_standard_endpoints),
            )
            //
            // Staff
            //
            .service(
                web::scope(
                    format!("/{}", endpoints::shared::UrlScope::Staffs)
                        .as_str(),
                )
                .configure(staff_account_endpoints::configure),
            );

        // ? -------------------------------------------------------------------
        // ? Configure authentication elements
        //
        // Mycelium Auth
        //
        // ? -------------------------------------------------------------------
        let app = match auth_config.internal {
            OptionalConfig::Enabled(config) => {
                //
                // Configure OAuth2 Scope
                //
                debug!("Configuring Mycelium Internal authentication");
                app.app_data(web::Data::new(config.clone()))
            }
            _ => app,
        };

        // ? -------------------------------------------------------------------
        // ? Configure authentication elements
        //
        // Google OAuth2
        //
        // ? -------------------------------------------------------------------
        let mycelium_scope = match auth_config.google {
            OptionalConfig::Enabled(_) => {
                //
                // Configure OAuth2 Scope
                //
                debug!("Configuring Google authentication");
                let scope = mycelium_scope.service(
                    web::scope("/auth/google")
                        .configure(google_handlers::configure),
                );
                debug!("Google OAuth2 configuration done");
                scope
            }
            _ => mycelium_scope,
        };

        // ? -------------------------------------------------------------------
        // TODO: Do implement the Azure AD authentication
        //
        // ? Configure authentication elements
        //
        // Azure AD OAuth2
        //
        // ? -------------------------------------------------------------------
        // let mycelium_scope = match auth_config.azure {
        //     OptionalConfig::Enabled(_) => {
        //         //
        //         // Configure OAuth2 Scope
        //         //
        //         debug!("Configuring Azure AD authentication");
        //         let scope = mycelium_scope.service(
        //             web::scope("/auth/azure")
        //                 .configure(azure_handlers::configure),
        //         );
        //         debug!("Azure AD OAuth2 configuration done");
        //         scope
        //     }
        //     _ => mycelium_scope,
        // };

        app
            // ? ---------------------------------------------------------------
            // ? Configure Session
            //
            // https://docs.rs/actix-session/latest/actix_session/storage/struct.CookieSessionStore.html
            //
            // ? ---------------------------------------------------------------
            .wrap(session_middleware())
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
                Logger::default()
                    .exclude_regex("/health/*")
                    .exclude_regex("/swagger-ui/*"),
            )
            // ? ---------------------------------------------------------------
            // ? Configure Injection modules
            // ? ---------------------------------------------------------------
            .configure(configure_injection_modules)
            // ? ---------------------------------------------------------------
            // ? Configure mycelium routes
            // ? ---------------------------------------------------------------
            .service(mycelium_scope)
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
                            Url::with_primary(
                                "Manager Users Endpoints",
                                "/doc/manager-openapi.json",
                                true,
                            ),
                            ManagerApiDoc::openapi(),
                        ),
                        (
                            Url::new(
                                "Standard Users Endpoints",
                                "/doc/default-users-openapi.json",
                            ),
                            StandardUsersApiDoc::openapi(),
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
