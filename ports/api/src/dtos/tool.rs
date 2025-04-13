use myc_core::domain::dtos::service::Service;
use mycelium_base::utils::errors::{execution_err, MappedErrors};
use serde::{Deserialize, Serialize};
use url::Url;
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

    /// The service openapi path
    ///
    /// Optional together with discoverable field. The path to the openapi.json
    /// file. The file should be used for external clients to discover the
    /// service. Is is used for the service discovery by LLM agents.
    ///
    pub openapi_path: String,
}

impl Tool {
    pub fn from_service(
        service: Service,
        host: Url,
    ) -> Result<Self, MappedErrors> {
        let openapi_path = if let Some(path) = service.openapi_path {
            path
        } else {
            return execution_err(format!(
                "OpenAPI path is not set for service: {}",
                service.name,
            ))
            .with_exp_true()
            .as_error();
        };

        let description = if let Some(desc) = service.description {
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
            openapi_path: format!(
                "{host}/{name}/{openapi_path}",
                host = host.to_string().trim_end_matches('/'),
                name = service.name.trim_end_matches('/'),
                openapi_path = openapi_path.trim_start_matches("/")
            ),
        })
    }
}
