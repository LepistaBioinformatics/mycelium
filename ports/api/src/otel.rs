use crate::models::api_config::{LogFormat, LoggingConfig, LoggingTarget};

use myc_core::domain::dtos::http::Protocol;
use opentelemetry::{global, trace::TracerProvider};
use opentelemetry_otlp::{WithExportConfig, WithHttpConfig, WithTonicConfig};
use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;
use tonic::metadata::{Ascii, MetadataKey, MetadataMap, MetadataValue};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt, layer::SubscriberExt, EnvFilter};

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
        // Populate config from execution environment
        //
        std::env::set_var("OTEL_SERVICE_NAME", name.to_owned());
        let headers = parse_otlp_headers_from_env();

        //
        // Build external address
        //
        let metrics_address =
            format!("{}://{}:{}/v1/metrics", protocol, host, port);

        let traces_address =
            format!("{}://{}:{}/v1/traces", protocol, host, port);

        //let logs_address = format!("{}://{}:{}/v1/logs", protocol, host, port);

        //
        // Initialize providers
        //
        let tracer_provider =
            opentelemetry_sdk::trace::SdkTracerProvider::builder();

        let meter_provider =
            opentelemetry_sdk::metrics::SdkMeterProvider::builder();

        //let logs_provider =
        //    opentelemetry_sdk::logs::SdkLoggerProvider::builder();

        let tracer_provider = (match protocol {
            Protocol::Grpc => {
                let trace_exporter =
                    opentelemetry_otlp::SpanExporter::builder()
                        .with_tonic()
                        .with_endpoint(traces_address)
                        .with_metadata(metadata_from_headers(headers))
                        .build()
                        .expect("Failed to build gRPC exporter");

                tracer_provider.with_simple_exporter(trace_exporter)
            }
            _ => {
                let trace_exporter =
                    opentelemetry_otlp::SpanExporter::builder()
                        .with_http()
                        .with_endpoint(traces_address)
                        .with_headers(headers)
                        .build()
                        .expect("Failed to build HTTP exporter");

                tracer_provider.with_simple_exporter(trace_exporter)
            }
        })
        .build()
        .tracer(name);

        let meter_provider = (match protocol {
            Protocol::Grpc => {
                let meter_exporter =
                    opentelemetry_otlp::MetricExporter::builder()
                        .with_tonic()
                        .with_endpoint(metrics_address)
                        .build()
                        .expect("Failed to build gRPC exporter");

                meter_provider.with_periodic_exporter(meter_exporter)
            }
            _ => {
                let meter_exporter =
                    opentelemetry_otlp::MetricExporter::builder()
                        .with_http()
                        .with_endpoint(metrics_address)
                        .build()
                        .expect("Failed to build HTTP exporter");

                meter_provider.with_periodic_exporter(meter_exporter)
            }
        })
        .build();

        let tracing_layer =
            tracing_opentelemetry::layer().with_tracer(tracer_provider);

        tracing_subscriber::Registry::default().with(tracing_layer);

        global::set_meter_provider(meter_provider);
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

/// Parse headers from environment variable into MetadataMap
///
/// This function is used to parse headers from environment variable
/// `OTEL_EXPORTER_OTLP_HEADERS` into MetadataMap. The headers are expected to
/// be in the format `name1=value1,name2=value2,...`. The function will return a
/// MetadataMap containing the headers.
fn metadata_from_headers(headers: HashMap<String, String>) -> MetadataMap {
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
fn parse_otlp_headers_from_env() -> HashMap<String, String> {
    let mut metadata = HashMap::new();

    if let Ok(hdrs) = std::env::var("OTEL_EXPORTER_OTLP_HEADERS") {
        hdrs.split(',')
            .map(|header| {
                header
                    .split_once('=')
                    .expect("Header should contain '=' character")
            })
            .for_each(|(name, value)| {
                metadata.insert(name.to_string(), value.to_string());
            });
    }

    metadata
}
