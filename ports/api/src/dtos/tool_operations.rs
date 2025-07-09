use myc_core::domain::dtos::{
    health_check_info::HealthStatus, http::HttpMethod,
    security_group::SecurityGroup,
};
use mycelium_openapi::Operation;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
pub(crate) struct ToolOperation {
    /// The path
    ///
    /// The openapi operation path. This should include the parent service
    /// name.
    ///
    pub path: String,

    /// The method
    ///
    /// The allowed method of the operation.
    ///
    pub method: HttpMethod,

    /// The operation
    ///
    /// A serialized operation. See the [Operation] struct for more details.
    ///
    #[serde(flatten)]
    pub operation: Operation,

    /// The mycelium security group
    ///
    /// The mycelium security group for the operation.
    ///
    pub security_group: SecurityGroup,

    /// The related service
    ///
    /// The related service of the operation.
    ///
    pub service: ServiceWrapper,
}

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
pub(crate) struct ServiceWrapper {
    /// The service name
    ///
    /// The name of the service.
    ///
    pub name: String,

    /// The service health status
    ///
    /// The health status of the service.
    ///
    pub health_status: HealthStatus,

    /// The service capabilities
    ///
    /// The capabilities of the service.
    ///
    pub capabilities: Option<Vec<String>>,

    /// The service description
    ///
    /// The description of the service. The description should be used during
    /// the service discovery by LLM agents.
    ///
    pub description: Option<String>,
}
