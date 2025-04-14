use myc_core::domain::entities::{ServiceRead, ServiceWrite};

#[tracing::instrument(name = "services_health_dispatcher", skip_all)]
pub(crate) fn services_health_dispatcher(
    service_read_repo: Box<&dyn ServiceRead>,
    service_write_repo: Box<&dyn ServiceWrite>,
) {
    unimplemented!()
}
