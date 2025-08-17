use crate::{
    dtos::{ServiceWrapper, ToolOperation},
    openapi_processor::load_paths_from_spec,
};

use myc_core::domain::dtos::service::Service;
use myc_mem_db::repositories::MemDbAppModule;
use mycelium_base::utils::errors::{execution_err, MappedErrors};
use mycelium_openapi::OpenApiSchema;
use std::{collections::HashMap, sync::Arc};

#[derive(Debug, Clone)]
pub struct ServiceOpenApiSchema {
    pub operations: Vec<ToolOperation>,
    pub docs: HashMap<String, OpenApiSchema>,
}

#[tracing::instrument(
    name = "load_operations_from_downstream_services",
    skip_all
)]
pub(super) async fn load_operations_from_downstream_services(
    services: Vec<Service>,
    app_modules: Arc<MemDbAppModule>,
) -> Result<ServiceOpenApiSchema, MappedErrors> {
    let mut operations = Vec::new();
    let mut docs = HashMap::new();

    for service in services {
        if let Some(false) = service.discoverable {
            continue;
        }

        if service.openapi_path.is_none() {
            continue;
        }

        let absolute_path = format!(
            "{protocol}://{host}/{path}",
            protocol = service.protocol.to_string(),
            host = service.host.choose_host(),
            path = service
                .openapi_path
                .ok_or(execution_err("OpenAPI path is not set"))?
                .strip_prefix("/")
                .unwrap()
        );

        tracing::debug!("Loading OpenAPI document from: {}", absolute_path);

        let response = reqwest::get(absolute_path).await.map_err(|e| {
            execution_err(format!("Failed to load from url: {}", e))
        })?;

        let content = response.text().await.map_err(|e| {
            execution_err(format!("Failed to read content from url: {}", e))
        })?;

        let doc = OpenApiSchema::load_doc_from_string(&content)?;

        let service_wrapper = ServiceWrapper {
            name: service.name.clone(),
            health_status: service.health_status,
            capabilities: service.capabilities,
            description: service.description,
        };

        let local_operations = load_paths_from_spec(
            doc.clone(),
            service_wrapper,
            app_modules.clone(),
            false,
        )
        .await?;

        operations.extend(local_operations);
        docs.insert(service.name, doc);
    }

    Ok(ServiceOpenApiSchema { operations, docs })
}
