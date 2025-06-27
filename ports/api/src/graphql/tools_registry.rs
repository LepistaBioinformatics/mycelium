use crate::{
    dtos::{ServiceWrapper, ToolOperation},
    graphql::{Components, OpenApiPartial},
};

use awc::http::uri::PathAndQuery;
use myc_core::{
    domain::{dtos::service::Service, entities::RoutesRead},
    use_cases::gateway::routes::match_forward_address,
};
use myc_mem_db::repositories::MemDbAppModule;
use mycelium_base::{
    entities::FetchResponseKind,
    utils::errors::{execution_err, MappedErrors},
};
use shaku::HasComponent;
use slugify::slugify;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct ToolsRegistry {
    pub operations: Vec<ToolOperation>,
    pub components: Vec<Components>,
}

impl ToolsRegistry {
    #[tracing::instrument(name = "load_from_services", skip_all)]
    pub async fn load_from_services(
        services: Vec<Service>,
        app_modules: Arc<MemDbAppModule>,
    ) -> Result<Self, MappedErrors> {
        let mut operations = Vec::new();
        let mut components = Vec::new();

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

            let doc = ToolsRegistry::load_doc_from_string(&content)?;

            let service_wrapper = ServiceWrapper {
                name: service.name,
                health_status: service.health_status.to_string(),
                capabilities: service.capabilities,
                description: service.description,
            };

            let paths = ToolsRegistry::load_paths_from_spec(
                doc.clone(),
                service_wrapper,
                app_modules.clone(),
            )
            .await?;

            operations.extend(paths.operations);
            components.extend(paths.components);
        }

        Ok(Self {
            operations,
            components,
        })
    }

    /// Loads the paths from a OpenAPI document
    #[tracing::instrument(name = "load_paths_from_spec", skip_all)]
    async fn load_paths_from_spec(
        doc: OpenApiPartial,
        service_wrapper: ServiceWrapper,
        app_modules: Arc<MemDbAppModule>,
    ) -> Result<Self, MappedErrors> {
        let routes_read_repo: &dyn RoutesRead = app_modules.resolve_ref();

        let mut operations = Vec::new();
        let mut components = Vec::new();
        let service_name = service_wrapper.name.to_owned();

        for (path, item) in doc.paths {
            let stripped_path = path.strip_prefix("/").unwrap_or(&path);

            //
            // Collect the security group from the path
            //
            let composed_path = format!("{}/{}", service_name, stripped_path);

            let path_and_query = PathAndQuery::try_from(composed_path)
                .map_err(|e| execution_err(format!("Invalid path: {}", e)))?;

            let route_forward = match match_forward_address(
                path_and_query.to_owned(),
                Box::new(routes_read_repo),
            )
            .await?
            {
                FetchResponseKind::Found(route) => route,
                FetchResponseKind::NotFound(path) => {
                    tracing::warn!(
                        "Route not covered for path: {}",
                        path.unwrap_or(path_and_query.to_string()).to_string()
                    );

                    continue;
                }
            };

            let security_group = serde_json::to_string(
                &route_forward.security_group,
            )
            .map_err(|e| {
                execution_err(format!(
                    "Failed to serialize security group: {}",
                    e
                ))
            })?;

            //
            // Populate the operations
            //
            for (method, operation) in item.iter() {
                let operation = if let Some(operation) = operation {
                    operation
                } else {
                    continue;
                };

                //
                // Collect basic openapi information
                //
                let slug_operation_id =
                    slugify!(&format!("{}-{}", service_name, path));

                let operation_id =
                    operation.operation_id.clone().unwrap_or(slug_operation_id);

                let summary = operation.summary.clone();
                let description = operation.description.clone();

                //
                // Collect method
                //
                // Methods should be allowed by the route definition
                //
                let method = method.to_string().to_uppercase();

                if !route_forward.methods.iter().any(|m| {
                    let str_m = m.to_string().to_uppercase();

                    if str_m == "ALL" {
                        return true;
                    }

                    str_m == method
                }) {
                    continue;
                }

                operations.push(ToolOperation {
                    service: service_wrapper.clone(),
                    operation_id: operation_id,
                    method: method.clone(),
                    path: format!("/{}/{}", service_name, stripped_path),
                    summary: summary.clone(),
                    description: description.clone(),
                    operation: operation.clone(),
                    security_group: security_group.clone(),
                });

                components.push(doc.components.clone());
            }
        }

        Ok(Self {
            operations,
            components,
        })
    }

    /// Loads a OpenAPI document from a string
    #[tracing::instrument(name = "load_doc_from_string", skip_all)]
    fn load_doc_from_string(
        content: &str,
    ) -> Result<OpenApiPartial, MappedErrors> {
        let doc =
            serde_json::from_str::<OpenApiPartial>(&content).map_err(|e| {
                execution_err(format!("Failed to parse OpenAPI document: {e}"))
            })?;

        Ok(doc)
    }
}

pub(super) mod open_api_schema {}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_spec_example_file() -> &'static str {
        include_str!("./mock/example-openapi.json")
    }

    #[test]
    fn test_load_doc_from_string() {
        let doc = ToolsRegistry::load_doc_from_string(get_spec_example_file());
        assert!(doc.is_ok());
    }
}
