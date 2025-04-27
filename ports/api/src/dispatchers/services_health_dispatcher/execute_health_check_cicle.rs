use crate::{
    dispatchers::services_health_dispatcher::check_single_service_health::check_single_service_health,
    settings::MYC_OPERATION_CODE,
};

use futures::future::join_all;
use myc_core::domain::entities::{ServiceRead, ServiceWrite};
use myc_http_tools::models::api_otel_codes::APIOtelCodes;
use mycelium_base::{
    entities::FetchManyResponseKind, utils::errors::MappedErrors,
};
use tracing::Instrument;

pub(super) enum ServiceHealthRunStatus {
    Continue,
    Stop,
}

#[tracing::instrument(
    name = "execute_health_check_cicle", 
    skip_all,
    fields(
        myc.hc.health_check_cicle_status = tracing::field::Empty,
    )
)]
pub(super) async fn execute_health_check_cicle(
    max_retry_count: u32,
    max_instances: u32,
    service_read_repo: Box<&dyn ServiceRead>,
    service_write_repo: Box<&dyn ServiceWrite>,
) -> Result<ServiceHealthRunStatus, MappedErrors> {
    let span = tracing::Span::current();

    tracing::trace!(
        { MYC_OPERATION_CODE } = ?APIOtelCodes::HC00002,
        "Starting services health check cicle"
    );

    //
    // Fetch services
    //
    // Fetching without filters to collect all downstream services.
    //
    let services_response = match service_read_repo
        .list_services(None, None, None)
        .instrument(span.clone())
        .await
    {
        Ok(services) => services,
        Err(err) => {
            span.record(
                "myc.hc.health_check_cicle_status",
                tracing::field::display("failed"),
            );

            tracing::error!(
                "Error on fetch services during services health dispatcher: {err}"
            );

            return Ok(ServiceHealthRunStatus::Stop);
        }
    };

    let services = match services_response {
        FetchManyResponseKind::Found(services) => services,
        FetchManyResponseKind::NotFound => {
            span.record(
                "myc.hc.health_check_cicle_status",
                tracing::field::display("aborted"),
            );

            tracing::trace!(
                "No services found during services health dispatcher"
            );

            return Ok(ServiceHealthRunStatus::Continue);
        }
        FetchManyResponseKind::FoundPaginated { records, count, .. } => {
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

    tracing::trace!("Evaluating health of {} services", services.len());

    //
    // Check services health
    //
    // In parallel, check the health of all downstream services.
    //
    let health_checks = join_all(services.into_iter().map(|service| {
        check_single_service_health(
            service.clone(),
            max_retry_count,
            max_instances,
            service_write_repo.clone(),
        )
        .instrument(span.clone())
    }))
    .await;

    for health_check in health_checks {
        if let Err(err) = health_check {
            tracing::error!(
                "Error on check service health during services health dispatcher: {err}"
            );
        }
    }

    span.record(
        "myc.hc.health_check_cicle_status",
        tracing::field::display("success"),
    );

    tracing::trace!(
        { MYC_OPERATION_CODE } = ?APIOtelCodes::HC00003,
        "Finished services health check cicle"
    );

    Ok(ServiceHealthRunStatus::Continue)
}
