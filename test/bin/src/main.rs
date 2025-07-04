mod api_docs;
mod endpoints;

use crate::{
    api_docs::ApiDoc,
    endpoints::{
        account_created_webhook, account_deleted_webhook,
        account_updated_webhook, expects_headers, health, protected,
        protected_by_role, protected_by_role_with_permission,
        protected_by_service_token_with_scope, public,
        test_authorization_header, test_query_parameter_token,
    },
};

use actix_web::{App, HttpServer};
use std::env::var_os;
use tracing_actix_web::TracingLogger;
use utoipa::OpenApi;
use utoipa_redoc::{FileConfig, Redoc, Servable};
use utoipa_swagger_ui::SwaggerUi;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    //
    // Configure utoipa redoc config file
    //
    if let Err(err) = std::env::var("UTOIPA_REDOC_CONFIG_FILE") {
        tracing::trace!("Error on get env `UTOIPA_REDOC_CONFIG_FILE`: {err}");
        tracing::info!("Env variable `UTOIPA_REDOC_CONFIG_FILE` not set. Setting default value");

        std::env::set_var(
            "UTOIPA_REDOC_CONFIG_FILE",
            "ports/api/src/api_docs/redoc.config.json",
        );
    }

    //
    // Configure service
    //
    let address = (
        "0.0.0.0",
        match var_os("SERVICE_PORT") {
            Some(path) => path
                .into_string()
                .unwrap_or("8080".to_string())
                .parse::<u16>()
                .unwrap(),
            None => 8080,
        },
    );

    //
    // Configure tracing
    //
    tracing_subscriber::fmt::init();

    //
    // Fire up the server
    //
    HttpServer::new(|| {
        App::new()
            .wrap(TracingLogger::default())
            .service(Redoc::with_url_and_config(
                "/doc/redoc",
                ApiDoc::openapi(),
                FileConfig,
            ))
            .service(
                SwaggerUi::new("/doc/swagger/{_:.*}")
                    .url("/doc/openapi.json", ApiDoc::openapi()),
            )
            .service(health)
            .service(public)
            .service(protected)
            .service(protected_by_role)
            .service(protected_by_role_with_permission)
            .service(protected_by_service_token_with_scope)
            .service(account_created_webhook)
            .service(account_updated_webhook)
            .service(account_deleted_webhook)
            .service(expects_headers)
            .service(test_query_parameter_token)
            .service(test_authorization_header)
    })
    .bind(address)?
    .workers(1)
    .run()
    .await
}
