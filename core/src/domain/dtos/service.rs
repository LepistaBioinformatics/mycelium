use super::{
    health_check_info::HealthStatus, http::Protocol, http_secret::HttpSecret,
    route::Route,
};

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
    /// host: localhost:8080
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
    ///   - localhost:8080
    ///   - localhost:8081
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

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ServiceType {
    RestApi,
    Unknown,
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
    #[serde(default = "default_id")]
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

    /// The service protocol
    ///
    /// The protocol of the service.
    ///
    #[serde(default = "default_protocol")]
    pub protocol: Protocol,

    /// The service routes
    ///
    /// The routes of the service.
    ///
    pub routes: Vec<Route>,

    /// The health status of the service
    ///
    #[serde(default = "default_health_status")]
    pub health_status: HealthStatus,

    /// The service health check configuration
    ///
    /// The health check configuration for the service.
    ///
    pub health_check_path: String,

    /// The service discoverable
    ///
    /// When true, the service will be discovered by LLM agents.
    ///
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discoverable: Option<bool>,

    /// The service type
    ///
    /// Optional together with discoverable field. The type of the service.
    ///
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_type: Option<ServiceType>,

    /// If is a context api
    ///
    /// If is a context api, the service will be discovered by LLM agents.
    ///
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_context_api: Option<bool>,

    /// The service capabilities
    ///
    /// Optional together with discoverable field. The capabilities of the
    /// service.
    ///
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<Vec<String>>,

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

    /// The service secrets
    ///
    /// The secrets of the service. Secrets are used to authenticate the api
    /// gateway at the downstream service. Individual routes can request a
    /// specific secret of this secrets vector.
    ///
    #[serde(skip_serializing_if = "Option::is_none", alias = "secret")]
    pub secrets: Option<Vec<ServiceSecret>>,

    /// The allowed sources
    ///
    /// A list of sources with permissions to access the downstream service.
    /// Values can be a domain name, ip address, a cidr block or a wildcard
    /// domain name.
    ///
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_sources: Option<Vec<String>>,

    /// The proxy address
    ///
    /// The proxy address of the service. This is used to forward requests to
    /// the service through a proxy. If the service is not behind a proxy, this
    /// field should be empty.
    ///
    /// Example:
    ///
    /// ```yaml
    /// proxyAddress: http://proxy.example.com:8080
    /// ```
    ///
    /// Then, the downstream url should be:
    ///
    /// ```bash
    /// http://proxy.example.com:8080/http://service.example.com:8080/api/v1/service/1234567890
    /// ```
    ///
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proxy_address: Option<String>,
}

fn default_protocol() -> Protocol {
    Protocol::Http
}

fn default_id() -> Uuid {
    Uuid::new_v4()
}

fn default_health_status() -> HealthStatus {
    HealthStatus::Unknown
}

impl Service {
    pub fn update_health_status(&mut self, health_status: HealthStatus) {
        self.health_status = health_status;
    }

    pub fn is_context_api(&self) -> bool {
        if let Some(is_context_api) = self.is_context_api {
            is_context_api
        } else {
            false
        }
    }
}
