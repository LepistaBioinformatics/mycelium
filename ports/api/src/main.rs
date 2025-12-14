mod api_docs;
mod dispatchers;
mod dtos;
mod endpoints;
//mod mcp;
mod middleware;
mod models;
mod modifiers;
mod openapi_processor;
mod otel;
mod router;
mod settings;

use crate::openapi_processor::initialize_tools_registry;

use actix_cors::Cors;
use actix_web::{
    dev::Service,
    middleware::{Logger, NormalizePath, TrailingSlash},
    web, App, HttpServer,
};
use actix_web_opentelemetry::RequestTracing;
use api_docs::ApiDoc;
use awc::{error::HeaderValue, Client};
use dispatchers::{
    email_dispatcher, services_health_dispatcher, webhook_dispatcher,
};
use endpoints::{
    index::heath_check_endpoints,
    manager::{
        account_endpoints as manager_account_endpoints,
        guest_role_endpoints as manager_guest_role_endpoints,
        tenant_endpoints as manager_tenant_endpoints,
    },
    openid::well_known_endpoints,
    role_scoped::configure as configure_standard_endpoints,
    service::tools_endpoints as service_tools_endpoints,
    shared::insert_role_header,
    staff::account_endpoints as staff_account_endpoints,
};
use models::config_handler::ConfigHandler;
use myc_adapters_shared_lib::models::{
    SharedAppModule, SharedClientImpl, SharedClientImplParameters,
    SharedClientProvider,
};
use myc_config::{
    init_vault_config_from_file, optional_config::OptionalConfig,
};
use myc_core::domain::entities::{
    LocalMessageReading, LocalMessageWrite, RemoteMessageWrite,
};
use myc_diesel::repositories::{
    DieselDbPoolProvider, DieselDbPoolProviderParameters, SqlAppModule,
};
use myc_http_tools::settings::DEFAULT_REQUEST_ID_KEY;
use myc_kv::repositories::KVAppModule;
use myc_mem_db::{
    models::config::DbPoolProvider,
    repositories::{
        MemDbAppModule, MemDbPoolProvider, MemDbPoolProviderParameters,
    },
};
use myc_notifier::{
    models::ClientProvider,
    repositories::{
        NotifierAppModule, NotifierClientImpl, NotifierClientImplParameters,
    },
};
use oauth2::http::HeaderName;
use openssl::{
    pkey::PKey,
    ssl::{SslAcceptor, SslMethod},
    x509::X509,
};
use otel::initialize_otel;
use reqwest::header::{
    ACCEPT, ACCESS_CONTROL_ALLOW_CREDENTIALS, ACCESS_CONTROL_ALLOW_METHODS,
    ACCESS_CONTROL_ALLOW_ORIGIN, CONTENT_LENGTH, CONTENT_TYPE,
};
use router::route_request;
use settings::{ADMIN_API_SCOPE, TOOLS_API_SCOPE};
use shaku::HasComponent;
use std::{path::PathBuf, str::FromStr, sync::Arc, sync::Mutex};
use tracing::{info, trace, Instrument};
use tracing_actix_web::TracingLogger;
use utoipa::OpenApi;
use utoipa_redoc::{FileConfig, Redoc, Servable};
use utoipa_swagger_ui::{oauth, Config, SwaggerUi};
use uuid::Uuid;

#[tokio::main]
pub async fn main() -> std::io::Result<()> {
    // ? -----------------------------------------------------------------------
    // ? EXPORT THE UTOIPA_REDOC_CONFIG_FILE ENVIRONMENT VARIABLE
    //
    // The UTOIPA_REDOC_CONFIG_FILE environment variable should be exported
    // before the server starts. The variable should contain the path to the
    // redoc configuration file.
    //
    // ? -----------------------------------------------------------------------

    if let Err(err) = std::env::var("UTOIPA_REDOC_CONFIG_FILE") {
        trace!("Error on get env `UTOIPA_REDOC_CONFIG_FILE`: {err}");
        info!("Env variable `UTOIPA_REDOC_CONFIG_FILE` not set. Setting default value");

        unsafe {
            std::env::set_var(
                "UTOIPA_REDOC_CONFIG_FILE",
                "ports/api/src/api_docs/redoc.config.json",
            );
        }
    }

    // ? -----------------------------------------------------------------------
    // ? INITIALIZE SERVICES CONFIGURATION
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
    // ? CONFIGURE LOGGING AND TELEMETRY
    //
    // The logging and telemetry configuration should be initialized before the
    // server starts. The configuration should be set to the server and the
    // server should be started.
    //
    // IMPORTANT: Does not remove the _guard variable from this context, because
    // it is used to keep the telemetry alive.
    //
    // ? -----------------------------------------------------------------------
    info!("Initializing Logging and Telemetry configuration");
    let _guard = initialize_otel(api_config.to_owned().logging)?;

    let span = tracing::Span::current();

    // ? -----------------------------------------------------------------------
    // ? INITIALIZE VAULT CONFIGURATION
    //
    // The vault configuration should be initialized before the server starts.
    // Vault configurations should be used to store sensitive data.
    //
    // ? -----------------------------------------------------------------------
    info!("Initializing Vault configs");
    init_vault_config_from_file(None, Some(config.vault))
        .instrument(span.to_owned())
        .await;

    // ? -----------------------------------------------------------------------
    // ? CONFIGURE INTERNAL DEPENDENCIES
    // ? -----------------------------------------------------------------------
    info!("Initialize internal dependencies");

    let sql_module = Arc::new(
        SqlAppModule::builder()
            .with_component_parameters::<DieselDbPoolProvider>(
                DieselDbPoolProviderParameters {
                    pool: DieselDbPoolProvider::new(
                        &match config
                            .diesel
                            .database_url
                            .async_get_or_error()
                            .await
                        {
                            Ok(url) => url,
                            Err(err) => {
                                panic!("Error on get database url: {err}")
                            }
                        }
                        .as_str(),
                    ),
                },
            )
            .build(),
    );

    let shared_provider =
        match SharedClientImpl::new(config.redis.to_owned()).await {
            Ok(provider) => provider,
            Err(err) => panic!("Error on initialize shared provider: {err}"),
        };

    let shared_module = Arc::new(
        SharedAppModule::builder()
            .with_component_parameters::<SharedClientImpl>(
                SharedClientImplParameters {
                    redis_client: shared_provider.get_redis_client(),
                    redis_config: shared_provider.get_redis_config(),
                },
            )
            .build(),
    );

    let notifier_provider = match NotifierClientImpl::new(
        config.queue.to_owned(),
        config.redis.to_owned(),
        config.smtp.to_owned(),
    )
    .await
    {
        Ok(provider) => provider,
        Err(err) => panic!("Error on initialize notifier provider: {err}"),
    };

    let notifier_module = Arc::new(
        NotifierAppModule::builder()
            .with_component_parameters::<SharedClientImpl>(
                SharedClientImplParameters {
                    redis_client: shared_provider.get_redis_client(),
                    redis_config: shared_provider.get_redis_config(),
                },
            )
            .with_component_parameters::<NotifierClientImpl>(
                NotifierClientImplParameters {
                    smtp_client: notifier_provider.get_smtp_client(),
                    queue_config: notifier_provider.get_queue_config(),
                },
            )
            .build(),
    );

    let kv_module = Arc::new(
        KVAppModule::builder()
            .with_component_parameters::<SharedClientImpl>(
                SharedClientImplParameters {
                    redis_client: shared_provider.get_redis_client(),
                    redis_config: shared_provider.get_redis_config(),
                },
            )
            .build(),
    );

    for service in config.api.services.clone() {
        trace!("Service: {:?}", service);
    }

    let mem_module = Arc::new(
        MemDbAppModule::builder()
            .with_component_parameters::<MemDbPoolProvider>(
                MemDbPoolProviderParameters {
                    services_db: Arc::new(Mutex::new(
                        MemDbPoolProvider::new(config.api.services.clone())
                            .await
                            .get_services_db(),
                    )),
                },
            )
            .build(),
    );

    // ? -----------------------------------------------------------------------
    // ? INITIALIZE THE TOOLS REGISTRY
    //
    // The tools registry should be initialized before the server starts. The
    // registry should be used to store the tools for the tools endpoints.
    //
    // ? -----------------------------------------------------------------------
    info!("Initializing tools registry");

    let tools_registry_schema = initialize_tools_registry(mem_module.clone())
        .instrument(span.to_owned())
        .await
        .map_err(|err| {
            tracing::error!("Error initializing tools registry: {err}");

            std::io::Error::new(std::io::ErrorKind::Other, err)
        })?;

    // ? -----------------------------------------------------------------------
    // ? FIRE THE EMAIL DISPATCHER
    //
    // The email dispatcher should be fired to allow emails to be sent.
    // Dispatching will occur in a separate thread.
    //
    // ? -----------------------------------------------------------------------
    info!("Fire email dispatcher");

    email_dispatcher(
        config.queue.to_owned(),
        unsafe {
            Arc::from_raw(*Arc::new(
                sql_module.resolve_ref() as &dyn LocalMessageReading
            ))
        },
        unsafe {
            Arc::from_raw(*Arc::new(
                sql_module.resolve_ref() as &dyn LocalMessageWrite
            ))
        },
        unsafe {
            Arc::from_raw(*Arc::new(
                notifier_module.resolve_ref() as &dyn RemoteMessageWrite
            ))
        },
    )
    .instrument(span.to_owned())
    .await;

    // ? -----------------------------------------------------------------------
    // ? FIRE THE WEBHOOK DISPATCHER
    //
    // The webhook dispatcher should be fired to allow webhooks to be dispatched.
    // Dispatching will occur in a separate thread.
    //
    // ? -----------------------------------------------------------------------
    info!("Fire webhook dispatcher");
    webhook_dispatcher(config.core.to_owned(), sql_module.clone())
        .instrument(span.to_owned())
        .await;

    // ? -----------------------------------------------------------------------
    // ? FIRE THE SERVICES HEALTH DISPATCHER
    //
    // The services health dispatcher should be fired to allow the services
    // health to be checked.
    //
    // ? -----------------------------------------------------------------------
    info!("Fire services health dispatcher");
    services_health_dispatcher(config.api.clone(), mem_module.clone())
        .instrument(span.to_owned())
        .await;

    // ? -----------------------------------------------------------------------
    // ? CONFIGURE THE SERVER
    // ? -----------------------------------------------------------------------
    info!("Startup the server configuration");
    let server = HttpServer::new(move || {
        //
        // Here we should clone the config to avoid borrowing issues
        //
        let allowed_origins = config.api.allowed_origins.clone();
        let forward_api_config = config.api.clone();
        let auth_config = config.auth.clone();
        let token_config = config.core.account_life_cycle.clone();

        //
        // Configure the CORS policy
        //
        let cors = Cors::default()
            .allowed_origin_fn(move |origin, _| {
                allowed_origins
                    .contains(&origin.to_str().unwrap_or("").to_string())
            })
            .expose_headers(vec![
                ACCESS_CONTROL_ALLOW_CREDENTIALS,
                ACCESS_CONTROL_ALLOW_METHODS,
                ACCESS_CONTROL_ALLOW_ORIGIN,
                CONTENT_LENGTH,
                CONTENT_TYPE,
                ACCEPT,
                HeaderName::from_str(DEFAULT_REQUEST_ID_KEY).unwrap(),
            ])
            .allow_any_header()
            .allow_any_method()
            .max_age(3600);

        trace!("Configured Cors: {:?}", cors);

        // ? -------------------------------------------------------------------
        // ? Create the basis for the application
        // ? -------------------------------------------------------------------
        let base_app = App::new()
            //
            // Configure CORS policies
            //
            .wrap(cors)
            //
            // Normalize path
            //
            .wrap(NormalizePath::new(TrailingSlash::MergeOnly))
            //
            // Configure tracing and logging
            //
            .wrap(RequestTracing::default())
            .wrap(TracingLogger::default())
            //
            // Inject configuration
            //
            .app_data(web::Data::new(tools_registry_schema.clone()))
            .app_data(web::Data::new(token_config).clone())
            .app_data(web::Data::new(auth_config.to_owned()).clone())
            //
            // Inject modules
            //
            .app_data(web::Data::from(sql_module.clone()))
            .app_data(web::Data::from(shared_module.clone()))
            .app_data(web::Data::from(notifier_module.clone()))
            .app_data(web::Data::from(kv_module.clone()))
            .app_data(web::Data::from(mem_module.clone()))
            //
            // Index endpoints
            //
            // Index endpoints allow to check the status of the service
            //
            .service(
                web::scope("/health")
                    .configure(heath_check_endpoints::configure),
            )
            //
            // The well known openid configuration path
            //
            // This path is used to get the well known openid configuration
            // from the auth0 server.
            //
            .configure(well_known_endpoints::configure)
            //
            // Configure tools routes
            //
            // These endpoints allow users to identify the status of public
            // services.
            //
            .service(
                web::scope(TOOLS_API_SCOPE)
                    .configure(service_tools_endpoints::configure),
            )
            //
            // Configure API documentation
            //
            .service(Redoc::with_url_and_config(
                "/doc/redoc",
                ApiDoc::openapi(),
                FileConfig,
            ))
            .service(
                SwaggerUi::new("/doc/swagger/{_:.*}")
                    .url("/doc/openapi.json", ApiDoc::openapi())
                    .oauth(
                        oauth::Config::new()
                            .client_id("client-id")
                            .scopes(vec![String::from("openid")])
                            .use_pkce_with_authorization_code_grant(true),
                    )
                    .config(
                        Config::default()
                            .filter(true)
                            .show_extensions(true)
                            .persist_authorization(true)
                            .show_common_extensions(true)
                            .request_snippets_enabled(true),
                    ),
            )
            //
            // Configure anti-log elements
            //
            // Filter docs and common routes from the logs.
            //
            .wrap(
                Logger::default()
                    .exclude_regex("/health/*")
                    .exclude_regex("/doc/swagger/*")
                    .exclude_regex("/doc/redoc/*"),
            );

        // ? -------------------------------------------------------------------
        // ? CREATE THE ADMIN SCOPE
        //
        // Here you can find endpoints for the mycelium management (admin
        // scope). There include super users endpoints endpoints and role scoped
        // endpoints.
        //
        // ? -------------------------------------------------------------------
        let admin_scope = web::scope(ADMIN_API_SCOPE)
            //
            // Super Users
            //
            // Super user endpoints allow to perform manage the staff and
            // manager users actions, including determine new staffs and
            // managers.
            //
            .service(
                web::scope(endpoints::shared::UrlScope::Staffs.str())
                    //
                    // Inject a header to be collected by the
                    // MyceliumProfileData extractor.
                    //
                    // An empty role header was injected to allow only the
                    // super users with Staff status to access the staff
                    // endpoints.
                    //
                    .wrap_fn(|req, srv| {
                        let req = insert_role_header(req, vec![]);

                        srv.call(req)
                    })
                    //
                    // Configure endpoints
                    //
                    .configure(staff_account_endpoints::configure),
            )
            //
            // Manager Users
            //
            .service(
                web::scope(endpoints::shared::UrlScope::Managers.str())
                    //
                    // Inject a header to be collected by the
                    // MyceliumProfileData extractor.
                    //
                    // An empty role header was injected to allow only the
                    // super users with Managers status to access the
                    // managers endpoints.
                    //
                    .wrap_fn(|req, srv| {
                        let req = insert_role_header(req, vec![]);

                        srv.call(req)
                    })
                    //
                    // Configure endpoints
                    //
                    .configure(manager_tenant_endpoints::configure)
                    .configure(manager_guest_role_endpoints::configure)
                    .configure(manager_account_endpoints::configure),
            )
            //
            // Role Scoped Endpoints
            //
            .configure(configure_standard_endpoints);

        // ? -------------------------------------------------------------------
        // ? CONFIGURE INTERNAL AUTHENTICATION
        // ? -------------------------------------------------------------------
        let final_app = match auth_config.internal {
            OptionalConfig::Enabled(config) => {
                //
                // Configure OAuth2 Scope
                //
                info!("Configuring Mycelium Internal authentication");
                base_app.app_data(web::Data::new(config.clone()))
            }
            _ => base_app,
        };

        // ? -------------------------------------------------------------------
        // ? CREATE THE GATEWAY SCOPE
        // ? -------------------------------------------------------------------
        final_app
            //
            // Configure admin routes
            //
            .service(admin_scope)
            //
            // Configure gateway routes
            //
            .app_data(web::Data::new(Client::new()))
            .app_data(web::Data::new(forward_api_config.to_owned()).clone())
            .wrap_fn(|mut req, srv| {
                req.headers_mut().insert(
                    HeaderName::from_str(DEFAULT_REQUEST_ID_KEY).unwrap(),
                    HeaderValue::from_str(Uuid::new_v4().to_string().as_str())
                        .unwrap(),
                );

                srv.call(req)
            })
            .default_service(web::to(route_request))
    });

    // ? -----------------------------------------------------------------------
    // ? FIRE THE SERVER
    // ? -----------------------------------------------------------------------

    let address = (
        api_config.to_owned().service_ip,
        api_config.to_owned().service_port,
    );

    info!("Listening on Address and Port: {:?}: ", address);

    // ? -----------------------------------------------------------------------
    // ? WITH TLS IF CONFIGURED
    // ? -----------------------------------------------------------------------
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

        //
        // Read the certificate content
        //
        let cert_pem = match tls_config.tls_cert.async_get_or_error().await {
            Ok(path) => path,
            Err(err) => panic!("Error on get TLS cert path: {err}"),
        };

        let cert = X509::from_pem(cert_pem.as_bytes())?;

        //
        // Read the certificate key
        //
        let key_pem = match tls_config.tls_key.async_get_or_error().await {
            Ok(path) => path,
            Err(err) => panic!("Error on get TLS key path: {err}"),
        };

        let key = PKey::private_key_from_pem(key_pem.as_bytes())?;

        //
        // Set the certificate and key
        //
        builder.set_certificate(&cert).unwrap();
        builder.set_private_key(&key).unwrap();

        info!("Fire the server with TLS");
        return server
            .bind_openssl(format!("{}:{}", address.0, address.1), builder)?
            .workers(api_config.service_workers.to_owned() as usize)
            .run()
            .await;
    }

    // ? -----------------------------------------------------------------------
    // ? WITHOUT TLS OTHERWISE
    // ? -----------------------------------------------------------------------
    info!("Fire the server without TLS");
    server
        .bind(address)?
        .workers(api_config.service_workers as usize)
        .run()
        .await
}
