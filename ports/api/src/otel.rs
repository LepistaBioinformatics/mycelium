use crate::models::api_config::{LogFormat, LoggingConfig, LoggingTarget};

use myc_core::domain::dtos::http::Protocol;
use opentelemetry::trace::TracerProvider;
use opentelemetry_otlp::WithExportConfig;
use std::path::PathBuf;
use std::str::FromStr;
use tonic::metadata::{Ascii, MetadataKey, MetadataMap, MetadataValue};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::prelude::*;
use tracing_subscriber::{fmt, EnvFilter};

/// Parse headers from environment variable into MetadataMap
///
/// This function is used to parse headers from environment variable
/// `OTEL_EXPORTER_OTLP_HEADERS` into MetadataMap. The headers are expected to
/// be in the format `name1=value1,name2=value2,...`. The function will return a
/// MetadataMap containing the headers.
fn metadata_from_headers(headers: Vec<(String, String)>) -> MetadataMap {
    let mut metadata = MetadataMap::new();

    headers.into_iter().for_each(|(name, value)| {
        let value = value
            .parse::<MetadataValue<Ascii>>()
            .expect("Header value invalid");
        metadata.insert(MetadataKey::from_str(&name).unwrap(), value);
    });

    metadata
}

/// Parse OTLP headers from environment variable
///
/// This function is used to parse headers from environment variable
/// `OTEL_EXPORTER_OTLP_HEADERS` into a vector of tuples. The headers are
/// expected to be in the format `name1=value1,name2=value2,...`. The function
/// will return a vector of tuples containing the headers.
fn parse_otlp_headers_from_env() -> Vec<(String, String)> {
    let mut headers = Vec::new();

    if let Ok(hdrs) = std::env::var("OTEL_EXPORTER_OTLP_HEADERS") {
        hdrs.split(',')
            .map(|header| {
                header
                    .split_once('=')
                    .expect("Header should contain '=' character")
            })
            .for_each(|(name, value)| {
                headers.push((name.to_owned(), value.to_owned()))
            });
    }
    headers
}

pub(super) fn initialize_otel(
    config: LoggingConfig,
) -> std::io::Result<WorkerGuard> {
    let (non_blocking, guard) = match config.target.to_owned() {
        //
        // If a log file is provided, log to the file
        //
        Some(LoggingTarget::File { path }) => {
            let mut log_file = PathBuf::from(path);

            let binding = log_file.to_owned();
            let parent_dir = binding
                .parent()
                .expect("Log file parent directory not found");

            match config.format {
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

    if let Some(LoggingTarget::Collector {
        name,
        protocol,
        host,
        port,
    }) = config.target
    {
        //
        // Jaeger logging configurations
        //
        std::env::set_var("OTEL_SERVICE_NAME", name.to_owned());
        let headers = parse_otlp_headers_from_env();
        let tracer = opentelemetry_otlp::new_pipeline().tracing();

        let address = format!("{}://{}:{}", protocol, host, port);

        let tracer = (match protocol {
            Protocol::Grpc => {
                let exporter = opentelemetry_otlp::new_exporter()
                    .tonic()
                    .with_endpoint(format!("{}/v1/logs", address))
                    .with_metadata(metadata_from_headers(headers));

                tracer.with_exporter(exporter)
            }
            _ => {
                let exporter = opentelemetry_otlp::new_exporter()
                    .http()
                    .with_endpoint(format!("{}/v1/logs", address))
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
                    .with_target(true)
                    .with_thread_ids(true)
                    .with_file(true)
                    .with_line_number(true)
                    .with_source_location(true),
            )
            .with_line_number(true)
            .with_writer(non_blocking)
            .with_env_filter(
                EnvFilter::from_str(config.level.as_str()).unwrap(),
            );

        match config.format {
            LogFormat::Ansi => tracing_formatting_layer.pretty().init(),
            LogFormat::Jsonl => tracing_formatting_layer.json().init(),
        };
    };

    Ok(guard)
}
