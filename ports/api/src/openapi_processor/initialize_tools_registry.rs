use super::load_operations_from_downstream_services::load_operations_from_downstream_services;
use crate::openapi_processor::load_operations_from_downstream_services::ServiceOpenApiSchema;

use myc_core::domain::entities::ServiceRead;
use myc_mem_db::repositories::MemDbAppModule;
use mycelium_base::{
    entities::FetchManyResponseKind,
    utils::errors::{execution_err, MappedErrors},
};
use shaku::HasComponent;
use std::sync::Arc;
use tracing::Instrument;

#[tracing::instrument(name = "initialize_tools_registry", skip_all)]
pub(crate) async fn initialize_tools_registry(
    app_modules: Arc<MemDbAppModule>,
) -> Result<ServiceOpenApiSchema, MappedErrors> {
    let span = tracing::Span::current();

    let service_read_repo: &dyn ServiceRead = app_modules.resolve_ref();
    let services = match service_read_repo
        .list_services(None, None, None)
        .instrument(span.clone())
        .await?
    {
        FetchManyResponseKind::Found(services) => services,
        _ => return execution_err("Failed to fetch services").as_error(),
    };

    load_operations_from_downstream_services(services, app_modules.clone())
        .instrument(span)
        .await
}
