use crate::{
    dispatchers::services_health_dispatcher::check_single_host_health::check_single_host_health,
    settings::MYC_OPERATION_CODE,
};

use myc_core::domain::{
    dtos::service::{Service, ServiceHost},
    entities::ServiceWrite,
};
use myc_http_tools::models::api_otel_codes::APIOtelCodes;
use mycelium_base::utils::errors::MappedErrors;
use tracing::Instrument;

#[tracing::instrument(
    name = "check_single_service_health",
    skip_all,
    fields(
        myc.hc.service_id = tracing::field::Empty,
        myc.hc.service_name = tracing::field::Empty,
    ),
)]
pub(super) async fn check_single_service_health(
    service: Service,
    max_retry_count: u32,
    max_instances: u32,
    service_write_repo: Box<&dyn ServiceWrite>,
) -> Result<(), MappedErrors> {
    let span = tracing::Span::current();

    span.record("myc.hc.service_id", tracing::field::display(service.id))
        .record(
            "myc.hc.service_name",
            tracing::field::display(service.name.clone()),
        );

    tracing::trace!(
        { MYC_OPERATION_CODE } = ?APIOtelCodes::HC00004,
        "Checking service health",
    );

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
        if let Err(err) = check_single_host_health(
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
        )
        .instrument(span.clone())
        .await
        {
            tracing::error!(
                "Error on check host health during services health dispatcher: {err}"
            );
        }
    }

    tracing::trace!(
        { MYC_OPERATION_CODE } = ?APIOtelCodes::HC00005,
        "Service {} health checked",
        service.name
    );

    Ok(())
}
