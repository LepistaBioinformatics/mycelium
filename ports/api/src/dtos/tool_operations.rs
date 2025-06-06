use crate::graphql::Operation;

use async_graphql::SimpleObject;
use serde::{Deserialize, Serialize};

#[derive(SimpleObject, Serialize, Deserialize, Clone, Debug)]
pub(crate) struct ToolOperation {
    /// The operation id
    ///
    /// The id of the operation.
    ///
    pub operation_id: String,

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
    pub method: String,

    /// The mycelium security group
    ///
    /// The mycelium security group for the operation.
    ///
    pub security_group: String,

    /// The summary
    ///
    /// The openapi summary of the operation.
    ///
    pub summary: Option<String>,

    /// The description
    ///
    /// The openapi description of the operation.
    ///
    pub description: Option<String>,

    /// The operation
    ///
    /// A serialized operation.
    ///
    pub operation: Operation,

    /// The related service
    ///
    /// The related service of the operation.
    ///
    pub service: ServiceWrapper,
}

#[derive(SimpleObject, Serialize, Deserialize, Clone, Debug)]
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
    pub health_status: String,

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
