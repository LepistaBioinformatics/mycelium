use crate::dtos::{ServiceWrapper, ToolOperation};

use awc::http::uri::PathAndQuery;
use myc_core::{
    domain::{
        dtos::{http::HttpMethod, service::Service},
        entities::RoutesRead,
    },
    use_cases::gateway::routes::match_forward_address,
};
use myc_mem_db::repositories::MemDbAppModule;
use mycelium_base::{
    entities::FetchResponseKind,
    utils::errors::{execution_err, MappedErrors},
};
use mycelium_openapi::OpenApiSchema;
use shaku::HasComponent;
use std::{collections::HashMap, str::FromStr, sync::Arc};

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
        )
        .await?;

        operations.extend(local_operations);
        docs.insert(service.name, doc);
    }

    Ok(ServiceOpenApiSchema { operations, docs })
}

#[tracing::instrument(name = "load_paths_from_spec", skip_all)]
async fn load_paths_from_spec(
    doc: OpenApiSchema,
    service_wrapper: ServiceWrapper,
    app_modules: Arc<MemDbAppModule>,
) -> Result<Vec<ToolOperation>, MappedErrors> {
    let routes_read_repo: &dyn RoutesRead = app_modules.resolve_ref();

    let mut operations = Vec::new();
    let service_name = service_wrapper.name.to_owned();

    for (path, item) in doc.paths.paths.iter() {
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

        //
        // Populate the operations
        //
        for (method, operation) in item.operations.iter() {
            //
            // Collect method
            //
            // Methods should be allowed by the route definition
            //
            let method = HttpMethod::from_str(method)
                .map_err(|e| execution_err(format!("Invalid method: {e}")))?;

            if !route_forward.methods.iter().any(|m| {
                let str_m = m.to_string().to_uppercase();

                if str_m == "ALL" || str_m == "NONE" {
                    return true;
                }

                str_m == method.to_string()
            }) {
                continue;
            }

            operations.push(ToolOperation {
                service: service_wrapper.clone(),
                method: method.clone(),
                path: format!("/{}/{}", service_name, stripped_path),
                operation: operation.clone(),
                security_group: route_forward.security_group.clone(),
            });
        }
    }

    Ok(operations)
}
