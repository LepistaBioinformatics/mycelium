use myc_config::{
    optional_config::OptionalConfig, secret_resolver::SecretResolver,
};
use myc_core::domain::dtos::{
    callback::{Callback, ExecutionMode},
    health_check_info::HealthStatus,
    http::Protocol,
    route::Route,
    service::{Service, ServiceHost, ServiceSecret, ServiceType},
};
use mycelium_base::utils::errors::{creation_err, MappedErrors};
use serde::{
    de::{MapAccess, Visitor},
    Deserialize, Deserializer,
};
use std::{fmt, path::PathBuf};
use toml;
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TlsConfig {
    pub tls_cert: SecretResolver<String>,
    pub tls_key: SecretResolver<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LogFormat {
    /// ANSI format
    ///
    /// This format is human-readable and colorful.
    Ansi,

    /// YAML format
    ///
    /// This format is machine-readable and can be used for log analysis.
    Jsonl,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LoggingTarget {
    Stdout,
    File {
        path: String,
    },
    Collector {
        name: String,
        protocol: Protocol,
        host: String,
        port: u32,
    },
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoggingConfig {
    pub level: String,
    pub format: LogFormat,
    pub target: Option<LoggingTarget>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheConfig {
    /// JWKS TTL
    ///
    /// The time to live for the JWKS cache.
    pub jwks_ttl: Option<u64>,

    /// Email TTL
    ///
    /// The time to live for the email cache.
    pub email_ttl: Option<u64>,

    /// Profile TTL
    ///
    /// The time to live for the profile cache.
    pub profile_ttl: Option<u64>,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            jwks_ttl: Some(60 * 60 * 12), // 12 hours
            email_ttl: Some(60 * 10),     // 10 minutes
            profile_ttl: Some(60 * 10),   // 10 minutes
        }
    }
}

/// Intermediate structure for deserializing Service without name field The name
/// will be filled from the map key [[service-name]]
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ServiceIntermediate {
    // All fields from Service except 'name' which comes from the map key
    #[serde(default = "default_service_id")]
    id: Uuid,
    #[serde(alias = "hosts")]
    host: ServiceHost,
    #[serde(default = "default_service_protocol")]
    protocol: Protocol,
    #[serde(alias = "path", default)]
    routes: Vec<Route>,
    #[serde(default = "default_service_health_status")]
    health_status: HealthStatus,
    health_check_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    discoverable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    service_type: Option<ServiceType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    is_context_api: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    capabilities: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    openapi_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", alias = "secret")]
    secrets: Option<Vec<ServiceSecret>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    allowed_sources: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    proxy_address: Option<String>,
}

fn default_service_id() -> Uuid {
    Uuid::new_v4()
}

fn default_service_protocol() -> Protocol {
    Protocol::Http
}

fn default_service_health_status() -> HealthStatus {
    HealthStatus::Unknown
}

/// Convert ServiceIntermediate to Service using the service key as the name
fn service_from_intermediate(
    service_key: String,
    intermediate: ServiceIntermediate,
) -> Service {
    Service {
        id: intermediate.id,
        name: service_key, // Always use the key from [[service-name]]
        host: intermediate.host,
        protocol: intermediate.protocol,
        routes: intermediate.routes,
        health_status: intermediate.health_status,
        health_check_path: intermediate.health_check_path,
        discoverable: intermediate.discoverable,
        service_type: intermediate.service_type,
        is_context_api: intermediate.is_context_api,
        capabilities: intermediate.capabilities,
        description: intermediate.description,
        openapi_path: intermediate.openapi_path,
        secrets: intermediate.secrets,
        allowed_sources: intermediate.allowed_sources,
        proxy_address: intermediate.proxy_address,
    }
}

/// Custom deserializer for services that accepts the ergonomic format:
/// [api.services] followed by [[service-name]]
///
/// After preprocessing, [[api.services.service-name]] creates
/// api.services.service-name as an array of tables, so api.services is a map
/// where each key is a service name and each value is Vec<ServiceIntermediate>
fn deserialize_services<'de, D>(
    deserializer: D,
) -> Result<Vec<Service>, D::Error>
where
    D: Deserializer<'de>,
{
    struct ServicesVisitor;

    impl<'de> Visitor<'de> for ServicesVisitor {
        type Value = Vec<Service>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a map of service arrays")
        }

        // Handle map format: [api.services] with [[service-name]] entries After
        // preprocessing, api.services is a map where each key is a service name
        // and each value is Vec<ServiceIntermediate> (from array of tables)
        fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
        where
            M: MapAccess<'de>,
        {
            let mut all_services = Vec::new();
            while let Some((service_key, services_vec)) =
                map.next_entry::<String, Vec<ServiceIntermediate>>()?
            {
                // Each service in the array
                for intermediate in services_vec {
                    // Convert using the service key as the name
                    let service = service_from_intermediate(
                        service_key.clone(),
                        intermediate,
                    );
                    all_services.push(service);
                }
            }
            Ok(all_services)
        }
    }

    deserializer.deserialize_map(ServicesVisitor)
}

/// Custom deserializer for callbacks that accepts the ergonomic format:
/// [api.callbacks] followed by [[callback]]
///
/// After preprocessing, [[api.callbacks.callback]] creates
/// api.callbacks.callback as an array of tables, so api.callbacks is a map
/// where the key is "callback" and the value is Vec<Callback>
fn deserialize_callbacks<'de, D>(
    deserializer: D,
) -> Result<Option<Vec<Callback>>, D::Error>
where
    D: Deserializer<'de>,
{
    struct CallbacksVisitor;

    impl<'de> Visitor<'de> for CallbacksVisitor {
        type Value = Option<Vec<Callback>>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a map of callback arrays")
        }

        // Handle map format: [api.callbacks] with [[callback]] entries
        // After preprocessing, api.callbacks is a map where the key is "callback"
        // and the value is Vec<Callback> (from array of tables)
        fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
        where
            M: MapAccess<'de>,
        {
            let mut all_callbacks = Vec::new();
            while let Some((_key, callbacks_vec)) =
                map.next_entry::<String, Vec<Callback>>()?
            {
                // Each callback in the array
                for callback in callbacks_vec {
                    all_callbacks.push(callback);
                }
            }
            if all_callbacks.is_empty() {
                Ok(None)
            } else {
                Ok(Some(all_callbacks))
            }
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(None)
        }

        fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: Deserializer<'de>,
        {
            deserializer.deserialize_map(CallbacksVisitor)
        }
    }

    deserializer.deserialize_option(CallbacksVisitor)
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiConfig {
    pub service_ip: String,
    pub service_port: u16,
    pub allowed_origins: Vec<String>,
    pub service_workers: i32,
    pub gateway_timeout: u64,
    pub logging: LoggingConfig,
    pub tls: OptionalConfig<TlsConfig>,
    pub cache: Option<CacheConfig>,
    pub health_check_interval: Option<u64>,
    pub max_retry_count: Option<u32>,
    pub max_error_instances: Option<u32>,

    #[serde(
        deserialize_with = "deserialize_callbacks",
        skip_serializing_if = "Option::is_none"
    )]
    pub callbacks: Option<Vec<Callback>>,

    #[serde(default)]
    pub callback_execution_mode: ExecutionMode,

    #[serde(deserialize_with = "deserialize_services")]
    pub services: Vec<Service>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TmpConfig {
    api: ApiConfig,
}

/// Pre-process TOML content to transform [[service-name]] into
/// [[api.services.service-name]] when inside [api.services] context
/// and [[callback]] into [[api.callbacks.callback]] when inside [api.callbacks] context
fn preprocess_toml_services(content: &str) -> String {
    let mut result = String::new();
    let mut in_services_context = false;
    let mut in_callbacks_context = false;

    for line in content.lines() {
        let trimmed = line.trim();

        // Check if we're entering [api.services] context
        if trimmed == "[api.services]" {
            in_services_context = true;
            in_callbacks_context = false;
            result.push_str(line);
            result.push('\n');
            continue;
        }

        // Check if we're entering [api.callbacks] context
        if trimmed == "[api.callbacks]" {
            in_callbacks_context = true;
            in_services_context = false;
            result.push_str(line);
            result.push('\n');
            continue;
        }

        // Check if we're leaving the services context (new top-level table)
        if in_services_context
            && trimmed.starts_with('[')
            && !trimmed.starts_with("[[")
        {
            // Check if it's not a sub-table of api.services
            if !trimmed.starts_with("[api.services.")
                && !trimmed.starts_with("[api.")
            {
                in_services_context = false;
            }
        }

        // Check if we're leaving the callbacks context (new top-level table)
        if in_callbacks_context
            && trimmed.starts_with('[')
            && !trimmed.starts_with("[[")
        {
            // Check if it's not a sub-table of api.callbacks
            if !trimmed.starts_with("[api.callbacks.")
                && !trimmed.starts_with("[api.")
            {
                in_callbacks_context = false;
            }
        }

        // Transform [[service-name]] to [[api.services.service-name]] when in
        // context
        if in_services_context
            && trimmed.starts_with("[[")
            && !trimmed.starts_with("[[api.services.")
        {
            // Extract the service name from [[service-name]] or
            // [[service-name.paths]]
            if let Some(start) = trimmed.find("[[") {
                if let Some(end) = trimmed[start + 2..].find("]]") {
                    let service_name = &trimmed[start + 2..start + 2 + end];
                    // Replace [[service-name with [[api.services.service-name
                    let new_line = line.replace(
                        &format!("[[{}", service_name),
                        &format!("[[api.services.{}", service_name),
                    );
                    result.push_str(&new_line);
                    result.push('\n');
                    continue;
                }
            }
        }

        // Transform [[callback]] to [[api.callbacks.callback]] when in context
        if in_callbacks_context
            && trimmed.starts_with("[[")
            && !trimmed.starts_with("[[api.callbacks.")
        {
            // Extract the callback name from [[callback]]
            if let Some(start) = trimmed.find("[[") {
                if let Some(end) = trimmed[start + 2..].find("]]") {
                    let callback_name = &trimmed[start + 2..start + 2 + end];
                    // Replace [[callback with [[api.callbacks.callback
                    let new_line = line.replace(
                        &format!("[[{}", callback_name),
                        &format!("[[api.callbacks.{}", callback_name),
                    );
                    result.push_str(&new_line);
                    result.push('\n');
                    continue;
                }
            }
        }

        result.push_str(line);
        result.push('\n');
    }

    result
}

impl ApiConfig {
    pub fn from_default_config_file(
        file: PathBuf,
    ) -> Result<Self, MappedErrors> {
        if !file.exists() {
            return creation_err(format!(
                "Could not find config file: {}",
                file.to_str().unwrap()
            ))
            .as_error();
        }

        // Read and preprocess the TOML file
        let file_content =
            std::fs::read_to_string(file.as_path()).map_err(|err| {
                creation_err(format!("Could not read config file: {err}"))
            })?;

        let preprocessed_content = preprocess_toml_services(&file_content);

        // Parse the preprocessed TOML
        let config: TmpConfig =
            toml::from_str(&preprocessed_content).map_err(|err| {
                creation_err(format!("Could not parse config file: {err}"))
            })?;

        Ok(config.api)
    }
}
