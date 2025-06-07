use crate::graphql::{QueryRoot, ToolsRegistry};

use async_graphql::{EmptyMutation, EmptySubscription, Schema};
use myc_core::domain::entities::ServiceRead;
use myc_mem_db::repositories::MemDbAppModule;
use mycelium_base::{
    entities::FetchManyResponseKind,
    utils::errors::{execution_err, MappedErrors},
};
use shaku::HasComponent;
use std::sync::Arc;

#[tracing::instrument(name = "initialize_tools_registry", skip_all)]
pub(crate) async fn initialize_tools_registry(
    app_modules: Arc<MemDbAppModule>,
) -> Result<Schema<QueryRoot, EmptyMutation, EmptySubscription>, MappedErrors> {
    let service_read_repo: &dyn ServiceRead = app_modules.resolve_ref();
    let services =
        match service_read_repo.list_services(None, None, None).await? {
            FetchManyResponseKind::Found(services) => services,
            _ => return execution_err("Failed to fetch services").as_error(),
        };

    let registry =
        ToolsRegistry::load_from_services(services, app_modules.clone())
            .await?;

    let schema = Schema::build(QueryRoot, EmptyMutation, EmptySubscription)
        .data(registry)
        .finish();

    Ok(schema)
}
