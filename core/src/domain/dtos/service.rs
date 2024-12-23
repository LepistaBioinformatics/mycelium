use super::{
    health_check::HealthCheckConfig, route::Route,
    service_secret::SecretReference,
};

use mycelium_base::dtos::UntaggedChildren;
use serde::{Deserialize, Serialize};
use utoipa::{ToResponse, ToSchema};
use uuid::Uuid;

/// The Upstream Service
///
/// The service is the upstream service that the route will proxy to.
///
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, ToResponse)]
#[serde(rename_all = "camelCase")]
pub struct Service {
    /// The service id
    pub id: Option<Uuid>,

    /// The service unique name
    pub name: String,

    /// The service host
    pub host: String,

    /// The service health check configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub health_check: Option<HealthCheckConfig>,

    /// The service routes
    pub routes: UntaggedChildren<Route, Uuid>,

    /// The service secrets
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secrets: Option<Vec<SecretReference>>,
}

impl Service {
    pub(crate) fn new(
        id: Option<Uuid>,
        name: String,
        host: String,
        health_check: Option<HealthCheckConfig>,
        routes: Vec<Route>,
        secrets: Option<Vec<SecretReference>>,
    ) -> Self {
        Self {
            id: match id {
                Some(id) => Some(id),
                None => {
                    Some(Uuid::new_v3(&Uuid::NAMESPACE_DNS, name.as_bytes()))
                }
            },
            name,
            host,
            health_check,
            routes: UntaggedChildren::Records(routes),
            secrets,
        }
    }
}
