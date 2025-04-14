use myc_core::domain::entities::{ServiceRead, ServiceWrite};

/// Check downstream services health
///
/// This function will dispatch a independent task to check the health of the
/// downstream services.
///
#[tracing::instrument(name = "services_health_dispatcher", skip_all)]
pub(crate) fn services_health_dispatcher(
    service_read_repo: Box<&dyn ServiceRead>,
    service_write_repo: Box<&dyn ServiceWrite>,
) {
    unimplemented!()
}
