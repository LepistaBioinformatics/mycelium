use crate::models::api_config::{LogFormat, LoggingConfig, LoggingTarget};

use myc_core::domain::dtos::http::Protocol;
use opentelemetry::{global, trace::TracerProvider, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::Resource;
use std::path::PathBuf;
use std::str::FromStr;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt, layer::SubscriberExt, EnvFilter, Layer};

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
        // Build external address
        //
        let (metrics_address, traces_address) = match protocol {
            Protocol::Grpc => (
                format!("http://{}:{}", host, port),
                format!("http://{}:{}", host, port),
            ),
            _ => (
                format!("{}://{}:{}/v1/metrics", protocol, host, port),
                format!("{}://{}:{}/v1/traces", protocol, host, port),
            ),
        };

        // ---------------------------------------------------------------------
        // Configure tracer
        // ---------------------------------------------------------------------

        let resource = Resource::builder()
            .with_attributes(vec![KeyValue::new(
                "service.name",
                name.to_owned(),
            )])
            .build();

        let tracer_provider =
            opentelemetry_sdk::trace::SdkTracerProvider::builder()
                .with_resource(resource.clone());

        let tracer_provider = (match protocol {
            Protocol::Grpc => {
                let trace_exporter =
                    opentelemetry_otlp::SpanExporter::builder()
                        .with_tonic()
                        .with_endpoint(traces_address)
                        .with_timeout(std::time::Duration::from_secs(10))
                        .build()
                        .expect("Failed to build gRPC exporter");

                tracer_provider.with_batch_exporter(trace_exporter)
            }
            _ => {
                let trace_exporter =
                    opentelemetry_otlp::SpanExporter::builder()
                        .with_http()
                        .with_endpoint(traces_address)
                        .with_timeout(std::time::Duration::from_secs(10))
                        .build()
                        .expect("Failed to build HTTP exporter");

                tracer_provider.with_batch_exporter(trace_exporter)
            }
        })
        .build();

        let tracer = tracer_provider.tracer(name.to_owned());

        global::set_tracer_provider(tracer_provider);

        let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);

        let subscriber = tracing_subscriber::Registry::default()
            .with(otel_layer)
            .with(
                tracing_subscriber::fmt::layer()
                    .with_writer(non_blocking)
                    .with_filter(
                        EnvFilter::from_str(config.level.as_str()).unwrap(),
                    ),
            );

        let _ = tracing::subscriber::set_global_default(subscriber);

        // ---------------------------------------------------------------------
        // Configure meter
        // ---------------------------------------------------------------------

        let meter_provider =
            opentelemetry_sdk::metrics::SdkMeterProvider::builder()
                .with_resource(resource.clone());

        let meter_provider = (match protocol {
            Protocol::Grpc => {
                let meter_exporter =
                    opentelemetry_otlp::MetricExporter::builder()
                        .with_tonic()
                        .with_endpoint(metrics_address)
                        .with_timeout(std::time::Duration::from_secs(10))
                        .build()
                        .expect("Failed to build gRPC exporter");

                meter_provider.with_periodic_exporter(meter_exporter)
            }
            _ => {
                let meter_exporter =
                    opentelemetry_otlp::MetricExporter::builder()
                        .with_http()
                        .with_endpoint(metrics_address)
                        .with_timeout(std::time::Duration::from_secs(10))
                        .build()
                        .expect("Failed to build HTTP exporter");

                meter_provider.with_periodic_exporter(meter_exporter)
            }
        })
        .build();

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
