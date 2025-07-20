use crate::dtos::{ServiceWrapper, ToolOperation};

use http::uri::PathAndQuery;
use myc_core::{
    domain::{
        dtos::{http::HttpMethod, security_group::SecurityGroup},
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
use std::{str::FromStr, sync::Arc};

/// Load the operations from the spec
///
/// This function loads the operations from the parsed openapi schema and
/// returns a vector of tool operations.
///
#[tracing::instrument(name = "load_paths_from_spec", skip_all)]
pub(crate) async fn load_paths_from_spec(
    doc: OpenApiSchema,
    service_wrapper: ServiceWrapper,
    app_modules: Arc<MemDbAppModule>,
    it_means_internal_route: bool,
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
            FetchResponseKind::Found(route) => Some(route),
            FetchResponseKind::NotFound(path) => {
                if !it_means_internal_route {
                    tracing::warn!(
                        "Route not covered for path: {}",
                        path.unwrap_or(path_and_query.to_string()).to_string()
                    );

                    continue;
                }

                None
            }
        };

        //
        // Populate the operations
        //
        for (method, operation) in item.operations.iter() {
            let method = HttpMethod::from_str(method)
                .map_err(|e| execution_err(format!("Invalid method: {e}")))?;

            //
            // Internal routes means that the route is not forwarded to an
            // external service. Thus, we should add a public operation for
            // the route.
            //
            if it_means_internal_route && route_forward.to_owned().is_none() {
                operations.push(ToolOperation {
                    service: service_wrapper.clone(),
                    method: method.clone(),
                    path: stripped_path.to_string(),
                    operation: operation.clone(),
                    security_group: SecurityGroup::Public,
                });

                continue;
            }

            //
            // Otherwise, we should check if the method is allowed by the
            // route.
            //
            let route_forward = route_forward.clone().unwrap();

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
