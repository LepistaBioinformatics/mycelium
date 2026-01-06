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
use opentelemetry::{global, trace::TracerProvider, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::Resource;
use std::env::var_os;
use std::str::FromStr;
use tracing_actix_web::TracingLogger;
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Layer};
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
    // Configure OpenTelemetry with hard coded values
    //
    // Hard coded configuration values
    let service_name = "mycelium-api-test-svc";
    let otel_collector_host = "myc-otel-collector-devcontainer";
    let otel_collector_port = 4317; // OTLP gRPC endpoint
    let log_level = "info";

    // Build OTLP endpoint for traces
    let traces_address =
        format!("grpc://{}:{}", otel_collector_host, otel_collector_port);

    // Configure resource
    let resource = Resource::builder()
        .with_attributes(vec![KeyValue::new("service.name", service_name)])
        .build();

    // ---------------------------------------------------------------------
    // Configure tracer
    // ---------------------------------------------------------------------

    let trace_exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(traces_address)
        .with_timeout(std::time::Duration::from_secs(10))
        .build()
        .expect("Failed to build gRPC trace exporter");

    let tracer_provider =
        opentelemetry_sdk::trace::SdkTracerProvider::builder()
            .with_resource(resource)
            .with_batch_exporter(trace_exporter)
            .build();

    let tracer = tracer_provider.tracer(service_name);

    global::set_tracer_provider(tracer_provider);

    // ---------------------------------------------------------------------
    // Configure tracing subscriber with OpenTelemetry layer
    // ---------------------------------------------------------------------

    let (non_blocking, _) = tracing_appender::non_blocking(std::io::stderr());

    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    let subscriber = tracing_subscriber::Registry::default()
        .with(otel_layer)
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(non_blocking)
                .with_filter(EnvFilter::from_str(log_level).unwrap()),
        );

    let _ = tracing::subscriber::set_global_default(subscriber);

    // Fire up the server
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
