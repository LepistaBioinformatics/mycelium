use crate::models::api_config::ApiConfig;

use futures::future::join_all;
use myc_core::domain::{
    dtos::{
        health_check_info::{HealthCheckInfo, HealthStatus, UnhealthyInstance},
        service::{Service, ServiceHost},
    },
    entities::{HealthCheckInfoWrite, ServiceRead, ServiceWrite},
};
use myc_diesel::repositories::SqlAppModule;
use myc_mem_db::repositories::MemDbAppModule;
use mycelium_base::{
    entities::FetchManyResponseKind,
    utils::errors::{execution_err, MappedErrors},
};
use rand::Rng;
use reqwest::{header::HeaderName, Response, StatusCode};
use shaku::HasComponent;
use std::{collections::HashMap, str::FromStr, sync::Arc, time::Duration};
use uuid::Uuid;

/// Check downstream services health
///
/// This function will dispatch a independent task to check the health of the
/// downstream services.
///
#[tracing::instrument(name = "services_health_dispatcher", skip_all)]
pub(crate) fn services_health_dispatcher(
    config: ApiConfig,
    sql_app_modules: Arc<SqlAppModule>,
    mem_app_modules: Arc<MemDbAppModule>,
) {
    tokio::spawn(async move {
        tracing::info!("Starting services health dispatcher");

        let inner_service_read_repo: &dyn ServiceRead =
            mem_app_modules.resolve_ref();
        let inner_service_write_repo: &dyn ServiceWrite =
            mem_app_modules.resolve_ref();
        let inner_health_check_info_write_repo: &dyn HealthCheckInfoWrite =
            sql_app_modules.resolve_ref();

        let mut interval = actix_rt::time::interval(Duration::from_secs(
            config.health_check_interval.unwrap_or(60 * 5),
        ));

        tracing::trace!(
            "Services health dispatcher interval: {} seconds",
            interval.period().as_secs()
        );

        let max_retry_count = config.max_retry_count.unwrap_or(3);
        let max_instances = config.max_error_instances.unwrap_or(5);

        //
        // Skip the first tick to avoid fetching events that were created in the
        // same second as the dispatcher start.
        //
        interval.tick().await;

        //
        // Wait for a random time between 1 and the consume interval. This time
        // should avoid the webhook dispatcher to start at the same time as the
        // email dispatcher and avoid the simultaneous consumption of the same
        // event over multiple containers.
        //
        let random_time =
            rand::thread_rng().gen_range(1..=interval.period().as_secs());

        tokio::time::sleep(Duration::from_secs(random_time)).await;

        loop {
            interval.tick().await;

            tracing::trace!("Checking services health");

            //
            // Fetch services
            //
            // Fetching without filters to collect all downstream services.
            //
            let services_response = match inner_service_read_repo
                .list_services(None, None, None)
                .await
            {
                Ok(services) => services,
                Err(err) => {
                    tracing::error!(
                        "Error on fetch services during services health dispatcher: {err}"
                    );

                    continue;
                }
            };

            let services = match services_response {
                FetchManyResponseKind::Found(services) => services,
                FetchManyResponseKind::NotFound => {
                    tracing::error!(
                        "No services found during services health dispatcher"
                    );

                    continue;
                }
                FetchManyResponseKind::FoundPaginated {
                    records,
                    count,
                    ..
                } => {
                    tracing::error!(
                        "Found paginated services during services health \
dispatcher. Health check will be performed for the first {len} services. \
Please, update the health check interval to return the full list of services \
instead of paginated. The full records count is {count}.",
                        len = records.len(),
                        count = count
                    );

                    records
                }
            };

            tracing::trace!("Services fetched: {:?}", services.len());

            //
            // Check services health
            //
            // In parallel, check the health of all downstream services.
            //
            let health_checks = join_all(services.into_iter().map(|service| {
                check_service_health(
                    service.clone(),
                    max_retry_count,
                    max_instances,
                    Box::new(inner_service_write_repo),
                    Box::new(inner_health_check_info_write_repo),
                )
            }))
            .await;

            for health_check in health_checks {
                if let Err(err) = health_check {
                    tracing::error!(
                        "Error on check service health during services health dispatcher: {err}"
                    );
                }
            }
        }
    });
}

#[tracing::instrument(name = "check_service_health", skip_all)]
async fn check_service_health(
    service: Service,
    max_retry_count: u32,
    max_instances: u32,
    service_write_repo: Box<&dyn ServiceWrite>,
    health_check_info_write_repo: Box<&dyn HealthCheckInfoWrite>,
) -> Result<(), MappedErrors> {
    tracing::trace!("Checking health for service: {}", service.name);

    // ? -----------------------------------------------------------------------
    // ? Check for service health
    //
    // If the service downstream route include multiple hosts, the health check
    // service downstream route include multiple hosts, the health check will be
    // performed for each host.
    //
    // ? -----------------------------------------------------------------------

    let hosts = match service.host {
        ServiceHost::Host(host) => vec![host],
        ServiceHost::Hosts(hosts) => hosts,
    };

    for host in hosts {
        check_host_health(
            service.id,
            service.name.clone(),
            service.health_status.clone(),
            format!(
                "{}://{}{}",
                service.protocol, host, service.health_check_path
            ),
            max_retry_count,
            max_instances,
            service_write_repo.clone(),
            health_check_info_write_repo.clone(),
        )
        .await?;
    }

    Ok(())
}

#[tracing::instrument(name = "check_host_health", skip_all)]
async fn check_host_health(
    service_id: Uuid,
    service_name: String,
    health_status: HealthStatus,
    host: String,
    max_retry_count: u32,
    max_instances: u32,
    service_write_repo: Box<&dyn ServiceWrite>,
    health_check_info_write_repo: Box<&dyn HealthCheckInfoWrite>,
) -> Result<(), MappedErrors> {
    tracing::trace!("Checking health for host: {}", host);

    // ? -----------------------------------------------------------------------
    // ? Check single host health
    //
    // Perform an HTTP GET request to the host and check the response status.
    //
    // ? -----------------------------------------------------------------------

    let mut response_time_ms = 0;
    let mut retry_count = match health_status {
        HealthStatus::Unknown | HealthStatus::Healthy { .. } => 0,
        HealthStatus::Unhealthy { attempts, .. } => attempts,
        HealthStatus::Unavailable { attempts, .. } => attempts,
    };

    let mut response = None;
    let mut timeout_occurred = false;
    let mut error = None;

    tracing::trace!("Checking host {host} health");

    for _ in 0..max_retry_count {
        retry_count += 1;

        let start_time = std::time::Instant::now();

        let local_response = match reqwest::get(host.to_owned()).await {
            Ok(response) => response,
            Err(err) => {
                error = Some(err);
                break;
            }
        };

        if local_response.status() == StatusCode::REQUEST_TIMEOUT {
            timeout_occurred = true;
            break;
        }

        response_time_ms = start_time.elapsed().as_millis();

        if local_response.status().is_success()
            || retry_count >= max_retry_count
        {
            response = Some(local_response);
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
        return Err(execution_err(format!(
            "Error on check host health with host {host}. Unable to perform the health check.",
            host = host,
        )))?;
    };

    // ? -----------------------------------------------------------------------
    // ? Update services health
    //
    // Update the in memory database of services. This will be used by
    // tools API to serve downstream services including their
    // availability.
    //
    // ? -----------------------------------------------------------------------

    service_write_repo
        .inform_health_status(
            service_id,
            service_name.clone(),
            match insident_level {
                0 => HealthStatus::set_health(health_check_info.checked_at),
                1 => HealthStatus::set_unhealthy(
                    health_status,
                    health_check_info.checked_at,
                    retry_count,
                    UnhealthyInstance {
                        host,
                        status_code: health_check_info.status_code,
                        response_body: health_check_info.response_body,
                        error_message: health_check_info.error_message,
                        checked_at: health_check_info.checked_at,
                    },
                    max_instances,
                ),
                2 => HealthStatus::set_unavailable(
                    health_check_info.checked_at,
                    retry_count,
                    health_check_info.error_message.unwrap_or_default(),
                ),
                _ => unreachable!(),
            },
        )
        .await?;

    // ? -----------------------------------------------------------------------
    // ? Register the health check info
    //
    // Register the health check info into the sql database.
    //
    // ? -----------------------------------------------------------------------

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
