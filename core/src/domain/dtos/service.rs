use super::{http_secret::HttpSecret, route::Route};

use myc_config::secret_resolver::SecretResolver;
use rand::seq::SliceRandom;
use serde::{ser::SerializeStruct, Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize, ToSchema, PartialEq, Eq)]
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

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
#[serde(untagged, rename_all = "camelCase")]
pub enum ServiceHost {
    /// The host of the service
    ///
    /// The host should include the port number.
    ///
    /// Example:
    ///
    /// ```yaml
    /// host: http://localhost:8080
    /// ```
    ///
    Host(String),

    /// The hosts of the service
    ///
    /// The hosts should include the port number.
    ///
    /// Example:
    ///
    /// ```yaml
    /// hosts:
    ///   - http://localhost:8080
    ///   - http://localhost:8081
    /// ```
    ///
    Hosts(Vec<String>),
}

impl ServiceHost {
    /// Random select a host if the host is a Hosts
    ///
    /// If the host is a Hosts, the function will return a random host from the
    /// vector.
    ///
    pub fn choose_host(&self) -> String {
        match self {
            ServiceHost::Host(host) => host.clone(),
            ServiceHost::Hosts(hosts) => {
                hosts.choose(&mut rand::thread_rng()).unwrap().clone()
            }
        }
    }
}

/// The Upstream Service
///
/// The service is the upstream service that the route will proxy to.
///
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Service {
    /// The service id
    ///
    /// The id of the service. If the id is not provided, the service will be
    /// generated using the name of the service.
    ///
    pub id: Uuid,

    /// The service unique name
    ///
    /// The name of the service. The name should be unique and is used to
    /// identify the service and call it from the gateway url path.
    ///
    pub name: String,

    /// The service host
    ///
    /// The host of the service. The host should include the port number. It
    /// can be a single host or a vector of hosts.
    ///
    #[serde(alias = "hosts")]
    pub host: ServiceHost,

    /// The service routes
    ///
    /// The routes of the service.
    ///
    pub routes: Vec<Route>,

    /// The service discoverable
    ///
    /// When true, the service will be discovered by LLM agents.
    ///
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discoverable: Option<bool>,

    /// The service description
    ///
    /// Optional together with discoverable field. The description of the
    /// service. The description should be used during the service discovery by
    /// LLM agents.
    ///
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// The service openapi path
    ///
    /// Optional together with discoverable field. The path to the openapi.json
    /// file. The file should be used for external clients to discover the
    /// service. Is is used for the service discovery by LLM agents.
    ///
    #[serde(skip_serializing_if = "Option::is_none")]
    pub openapi_path: Option<String>,

    /// The service health check configuration
    ///
    /// The health check configuration for the service.
    ///
    #[serde(skip_serializing_if = "Option::is_none")]
    pub health_check_path: Option<String>,

    /// The service secrets
    ///
    /// The secrets of the service. Secrets are used to authenticate the api
    /// gateway at the downstream service. Individual routes can request a
    /// specific secret of this secrets vector.
    ///
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secrets: Option<Vec<ServiceSecret>>,
}

impl Service {
    pub(crate) fn new(
        id: Option<Uuid>,
        name: String,
        host: ServiceHost,
        discoverable: Option<bool>,
        description: Option<String>,
        openapi_path: Option<String>,
        health_check_path: Option<String>,
        routes: Vec<Route>,
        secrets: Option<Vec<ServiceSecret>>,
    ) -> Self {
        //
        // If the service is discoverable, the description, health_check and
        // openapi_path are required.
        //
        if Some(true) == discoverable {
            for (name, param) in [
                ("description", description.is_none()),
                ("openapiPath", openapi_path.is_none()),
                ("healthCheckPath", health_check_path.is_none()),
            ] {
                if param {
                    panic!(
                        "The parameter '{name}' is required for discoverable services",
                    );
                }
            }
        }

        Self {
            id: match id {
                Some(id) => id,
                None => Uuid::new_v3(&Uuid::NAMESPACE_DNS, name.as_bytes()),
            },
            name,
            host,
            discoverable,
            description,
            openapi_path,
            health_check_path,
            routes,
            secrets,
        }
    }
}
