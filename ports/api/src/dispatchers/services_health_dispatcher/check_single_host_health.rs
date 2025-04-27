use crate::settings::{
    build_health_check_key, MYC_IS_HOST_HEALTHY, MYC_OPERATION_CODE,
};

use myc_core::domain::{
    dtos::health_check_info::{
        HealthCheckInfo, HealthStatus, UnhealthyInstance,
    },
    entities::ServiceWrite,
};
use myc_http_tools::models::api_otel_codes::APIOtelCodes;
use mycelium_base::utils::errors::{execution_err, MappedErrors};
use reqwest::{header::HeaderName, Response, StatusCode};
use std::{collections::HashMap, str::FromStr, time::Duration};
use tracing::Instrument;
use uuid::Uuid;

#[tracing::instrument(
    name = "check_single_host_health",
    skip_all,
    fields(
        myc.hc.host = tracing::field::Empty,
        myc.hc.retry_count = tracing::field::Empty,
        myc.hc.operation_code = tracing::field::Empty,
        myc.hc.health_check_cicle_status = tracing::field::Empty,
        myc.hc.is_host_healthy = tracing::field::Empty,
        myc.hc.status_code = tracing::field::Empty,
        myc.hc.is_service_healthy = tracing::field::Empty,
        myc.hc.response_time_ms = tracing::field::Empty,
        myc.hc.dns_resolved_ip = tracing::field::Empty,
        myc.hc.error_message = tracing::field::Empty,
        myc.hc.headers = tracing::field::Empty,
        myc.hc.response_body = tracing::field::Empty,
        myc.hc.content_type = tracing::field::Empty,
        myc.hc.response_size_bytes = tracing::field::Empty,
        myc.hc.timeout_occurred = tracing::field::Empty,
    ),
)]
pub(super) async fn check_single_host_health(
    service_id: Uuid,
    service_name: String,
    health_status: HealthStatus,
    host: String,
    max_retry_count: u32,
    max_instances: u32,
    service_write_repo: Box<&dyn ServiceWrite>,
) -> Result<(), MappedErrors> {
    let span = tracing::Span::current();

    span.record("myc.hc.host", tracing::field::display(host.clone()));

    tracing::trace!(
        { MYC_OPERATION_CODE } = ?APIOtelCodes::HC00006,
        "Checking host health",
    );

    // ? -----------------------------------------------------------------------
    // ? Check single host health
    //
    // Perform an HTTP GET request to the host and check the response status.
    //
    // ? -----------------------------------------------------------------------

    let mut response_time_ms = 0;
    let mut retry_count = 0;
    let mut response = None;
    let mut timeout_occurred = false;
    let mut error = None;

    for _ in 0..max_retry_count {
        retry_count += 1;

        if retry_count > max_retry_count {
            let message = format!(
                "Service {} with host {} health check failed after {} retries",
                service_name, host, retry_count
            );

            tracing::error!("{}", message);

            error = Some(message);
            break;
        }

        let start_time = std::time::Instant::now();

        let local_response = match reqwest::get(host.to_owned()).await {
            Ok(response) => response,
            Err(err) => {
                let msg = err.to_string();

                tracing::error!("{}", msg);

                error = Some(msg);
                tokio::time::sleep(Duration::from_secs(3)).await;

                continue;
            }
        };

        if local_response.status() == StatusCode::REQUEST_TIMEOUT {
            timeout_occurred = true;
            tokio::time::sleep(Duration::from_secs(3)).await;

            continue;
        }

        response_time_ms = start_time.elapsed().as_millis();

        if local_response.status().is_success() {
            response = Some(local_response);

            tracing::trace!(
                "Service {} with host {} health check passed",
                service_name,
                host
            );

            break;
        }

        tokio::time::sleep(Duration::from_secs(3)).await;
    }

    let (health_check_info, insident_level) = if let Some(response) = response {
        let parsed_response = parse_valid_http_response(
            service_id,
            service_name.clone(),
            response_time_ms as u64,
            retry_count,
            timeout_occurred,
            response,
        )
        .instrument(span.clone())
        .await?;

        let status = if parsed_response.is_service_healthy {
            0
        } else {
            1
        };

        (parsed_response, status)
    } else if let Some(error) = error {
        let parsed_response = HealthCheckInfo::new_when_unavailable(
            service_id,
            service_name.clone(),
            error.to_string(),
        );

        (parsed_response, 2)
    } else {
        return execution_err(format!(
            "Error on check host health with host {host}. Unable to perform the health check.",
            host = host,
        )).as_error();
    };

    // ? -----------------------------------------------------------------------
    // ? Update services health
    //
    // Update the in memory database of services. This will be used by
    // tools API to serve downstream services including their
    // availability.
    //
    // ? -----------------------------------------------------------------------

    let health_check_info_for_service = health_check_info.clone();

    service_write_repo
        .inform_health_status(
            service_id,
            service_name.clone(),
            match insident_level {
                0 => HealthStatus::set_health(
                    health_check_info_for_service.checked_at,
                ),
                1 => HealthStatus::set_unhealthy(
                    health_status,
                    health_check_info_for_service.checked_at,
                    retry_count,
                    UnhealthyInstance {
                        host,
                        status_code: health_check_info_for_service.status_code,
                        response_body: health_check_info_for_service
                            .response_body,
                        error_message: health_check_info_for_service
                            .error_message,
                        checked_at: health_check_info_for_service.checked_at,
                    },
                    max_instances,
                ),
                2 => HealthStatus::set_unavailable(
                    health_check_info_for_service.checked_at,
                    retry_count,
                    health_check_info_for_service
                        .error_message
                        .unwrap_or_default(),
                ),
                _ => unreachable!(),
            },
        )
        .instrument(span.clone())
        .await?;

    tracing::trace!(
        "Health check info for service {} with id {} published",
        service_name.clone(),
        service_id
    );

    // ? -----------------------------------------------------------------------
    // ? Register health check info
    //
    // Health check info is registered into otel collector using the span
    // context.
    //
    // ? -----------------------------------------------------------------------

    span.record(
        build_health_check_key("status_code").as_str(),
        tracing::field::display(health_check_info.status_code),
    )
    .record(
        build_health_check_key("is_service_healthy").as_str(),
        tracing::field::display(health_check_info.is_service_healthy),
    )
    .record(
        build_health_check_key("response_time_ms").as_str(),
        tracing::field::display(health_check_info.response_time_ms),
    )
    .record(
        build_health_check_key("dns_resolved_ip").as_str(),
        tracing::field::display(health_check_info.dns_resolved_ip.clone()),
    );

    let mut json_health_check_info = HashMap::new();

    if let Some(error_message) = health_check_info.error_message {
        json_health_check_info
            .insert("error_message".to_string(), error_message);
    }

    if let Some(headers) = health_check_info.headers {
        for (key, value) in headers {
            json_health_check_info
                .insert(format!("header.{key}"), value.to_string());
        }
    }

    if let Some(response_body) = health_check_info.response_body {
        json_health_check_info
            .insert("response_body".to_string(), response_body);
    }

    if let Some(response_size_bytes) = health_check_info.response_size_bytes {
        json_health_check_info.insert(
            "response_size_bytes".to_string(),
            response_size_bytes.to_string(),
        );
    }

    if let Some(retry_count) = health_check_info.retry_count {
        json_health_check_info
            .insert("retry_count".to_string(), retry_count.to_string());
    }

    if let Some(timeout_occurred) = health_check_info.timeout_occurred {
        json_health_check_info.insert(
            "timeout_occurred".to_string(),
            timeout_occurred.to_string(),
        );
    }

    tracing::trace!(
        { MYC_IS_HOST_HEALTHY } = health_check_info.is_service_healthy,
        myc.hc.response = ?json_health_check_info,
        "Host health check finished",
    );

    Ok(())
}

/// Parse valid http response
///
/// Used to parse response when the reqwest::get returns a valid response.
///
#[tracing::instrument(name = "parse_valid_http_response", skip_all)]
async fn parse_valid_http_response(
    service_id: Uuid,
    service_name: String,
    response_time_ms: u64,
    retry_count: u32,
    timeout_occurred: bool,
    response: Response,
) -> Result<HealthCheckInfo, MappedErrors> {
    let status_code = response.status().as_u16();

    let dns_resolved_ip = response
        .remote_addr()
        .map(|addr| addr.ip().to_string())
        .unwrap_or_default();

    //
    // Build the health check info based on the response status code
    //
    let health_check_info = {
        //
        // Evaluate for success
        //
        if status_code >= 200 && status_code < 300 {
            HealthCheckInfo::new_when_health(
                service_id,
                service_name.clone(),
                status_code,
                response_time_ms as u64,
                dns_resolved_ip,
            )
        //
        // Evaluate for failure
        //
        } else {
            let headers = response
                .headers()
                .clone()
                .into_iter()
                //
                // Filter to remove headers with empty values
                //
                .enumerate()
                .map(|(index, (key, value))| {
                    (
                        key.unwrap_or(
                            HeaderName::from_str(&format!("header_{index}"))
                                .unwrap(),
                        )
                        .to_string(),
                        value.to_str().unwrap_or_default().to_string(),
                    )
                })
                .collect::<HashMap<String, String>>();

            let content_type = response
                .headers()
                .get("content-type")
                .map(|value| value.to_str().unwrap_or_default())
                .unwrap_or_default()
                .to_string();

            let response_size_bytes =
                response.content_length().unwrap_or(0) as u64;

            let response_body = response.text().await.unwrap_or_default();

            HealthCheckInfo::new_when_unhealthy(
                service_id,
                service_name.clone(),
                status_code,
                response_time_ms as u64,
                dns_resolved_ip,
                response_body,
                headers,
                content_type,
                response_size_bytes,
                retry_count,
                timeout_occurred,
            )
        }
    };

    Ok(health_check_info)
}
