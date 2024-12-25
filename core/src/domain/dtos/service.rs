use super::{
    health_check::HealthCheckConfig, http_secret::HttpSecret, route::Route,
};

use myc_config::secret_resolver::SecretResolver;
use mycelium_base::dtos::UntaggedChildren;
use serde::{ser::SerializeStruct, Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ServiceSecret {
    pub(crate) name: String,

    #[serde(flatten)]
    pub(crate) secret: SecretResolver<HttpSecret>,
}

impl ServiceSecret {
    pub(crate) fn new(
        name: String,
        secret: SecretResolver<HttpSecret>,
    ) -> Self {
        Self { name, secret }
    }
}

impl Serialize for ServiceSecret {
    /// Serialize the secret
    ///
    /// The serialization should redact the secret value.
    ///
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        // Clone the secret
        let mut secret = self.secret.clone();

        // Redact the secret value if the ServiceResolver was a Value
        let secret_resolver = match &mut secret {
            SecretResolver::Value(secret) => {
                secret.redact_token();

                SecretResolver::Value(secret.clone())
            }
            other => other.clone(),
        };

        // Serialize the secret
        let mut state = serializer.serialize_struct("ServiceSecret", 2)?;
        state.serialize_field("name", &self.name)?;
        state.serialize_field("secret", &secret_resolver)?;
        state.end()
    }
}

/// The Upstream Service
///
/// The service is the upstream service that the route will proxy to.
///
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
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
    pub secrets: Option<Vec<ServiceSecret>>,
}

impl Service {
    pub(crate) fn new(
        id: Option<Uuid>,
        name: String,
        host: String,
        health_check: Option<HealthCheckConfig>,
        routes: Vec<Route>,
        secrets: Option<Vec<ServiceSecret>>,
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
