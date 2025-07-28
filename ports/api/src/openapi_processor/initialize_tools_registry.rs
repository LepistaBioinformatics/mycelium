use super::load_operations_from_downstream_services::load_operations_from_downstream_services;
use crate::{
    api_docs::ApiDoc,
    dtos::ServiceWrapper,
    openapi_processor::{
        load_operations_from_downstream_services::ServiceOpenApiSchema,
        load_paths_from_spec,
    },
};

use chrono::Local;
use myc_core::domain::{
    dtos::health_check_info::HealthStatus, entities::ServiceRead,
};
use myc_mem_db::repositories::MemDbAppModule;
use mycelium_base::{
    entities::FetchManyResponseKind,
    utils::errors::{execution_err, MappedErrors},
};
use mycelium_openapi::OpenApiSchema;
use shaku::HasComponent;
use std::{collections::HashMap, sync::Arc};
use tracing::Instrument;
use utoipa::OpenApi;

const MYCELIUM_SERVICE_NAME: &str = "MAG";

#[tracing::instrument(name = "initialize_tools_registry", skip_all)]
pub(crate) async fn initialize_tools_registry(
    app_modules: Arc<MemDbAppModule>,
) -> Result<ServiceOpenApiSchema, MappedErrors> {
    let span = tracing::Span::current();

    let mut service_operations = ServiceOpenApiSchema {
        operations: Vec::new(),
        docs: HashMap::new(),
    };

    // -------------------------------------------------------------------------
    // Load downstream services
    // -------------------------------------------------------------------------

    let service_read_repo: &dyn ServiceRead = app_modules.resolve_ref();
    let services = match service_read_repo
        .list_services(None, None, None)
        .instrument(span.clone())
        .await?
    {
        FetchManyResponseKind::Found(services) => services,
        _ => return execution_err("Failed to fetch services").as_error(),
    };

    let downstream_operations =
        load_operations_from_downstream_services(services, app_modules.clone())
            .instrument(span)
            .await?;

    service_operations
        .operations
        .extend(downstream_operations.operations);

    service_operations.docs.extend(downstream_operations.docs);

    // -------------------------------------------------------------------------
    // Load mycelium operations
    // -------------------------------------------------------------------------

    let mycelium_openapi_schema =
        ApiDoc::openapi().to_pretty_json().map_err(|e| {
            execution_err(format!(
                "Failed to convert mycelium operations to json: {e}"
            ))
        })?;

    let mycelium_docs =
        OpenApiSchema::load_doc_from_string(&mycelium_openapi_schema)?;

    let service_wrapper = ServiceWrapper {
        name: MYCELIUM_SERVICE_NAME.to_string(),
        health_status: HealthStatus::Healthy {
            checked_at: Local::now(),
        },
        capabilities: Some(
            vec!["identify-federation", "tenant", "account-management"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
        ),
        description: Some("Mycelium API Gateway".to_string()),
    };

    let mycelium_operations = load_paths_from_spec(
        mycelium_docs.clone(),
        service_wrapper,
        app_modules,
        true,
    )
    .await?;

    service_operations.operations.extend(mycelium_operations);
    service_operations
        .docs
        .insert(MYCELIUM_SERVICE_NAME.to_string(), mycelium_docs);

    Ok(service_operations)
}
