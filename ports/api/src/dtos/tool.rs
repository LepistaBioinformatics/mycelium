use myc_core::domain::dtos::{
    health_check_info::HealthStatus,
    service::{Service, ServiceType},
};
use mycelium_base::utils::errors::{execution_err, MappedErrors};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Tool {
    /// The service unique name
    ///
    /// The name of the service. The name should be unique and is used to
    /// identify the service and call it from the gateway url path.
    ///
    pub name: String,

    /// The service description
    ///
    /// Optional together with discoverable field. The description of the
    /// service. The description should be used during the service discovery by
    /// LLM agents.
    ///
    pub description: String,

    /// The service type
    ///
    /// The type of the service.
    ///
    pub tool_type: ServiceType,

    /// If is a context api
    ///
    /// If is a context api, the service will be discovered by LLM agents.
    ///
    pub is_context_api: bool,

    /// The service capabilities
    ///
    /// The capabilities of the service.
    ///
    pub capabilities: Vec<String>,

    /// The service openapi path
    ///
    /// Optional together with discoverable field. The path to the openapi.json
    /// file. The file should be used for external clients to discover the
    /// service. Is is used for the service discovery by LLM agents.
    ///
    pub openapi_path: String,

    /// The service health status
    ///
    /// The health status of the service.
    ///
    pub health_status: HealthStatus,
}

impl Tool {
    pub fn from_service(service: Service) -> Result<Self, MappedErrors> {
        let openapi_path = if let Some(path) = service.openapi_path.clone() {
            path
        } else {
            return execution_err(format!(
                "OpenAPI path is not set for service: {}",
                service.name,
            ))
            .with_exp_true()
            .as_error();
        };

        let description = if let Some(desc) = service.description.clone() {
            desc
        } else {
            return execution_err(format!(
                "Description is not set for service: {}",
                service.name,
            ))
            .with_exp_true()
            .as_error();
        };

        Ok(Self {
            name: service.name.clone(),
            description,
            capabilities: service.capabilities.clone().unwrap_or_default(),
            health_status: service.health_status.clone(),
            is_context_api: service.is_context_api(),
            openapi_path: format!(
                "/{name}/{openapi_path}",
                name = service.name.trim_end_matches('/'),
                openapi_path = openapi_path.trim_start_matches("/")
            ),
            tool_type: service
                .service_type
                .clone()
                .unwrap_or(ServiceType::Unknown),
        })
    }
}
