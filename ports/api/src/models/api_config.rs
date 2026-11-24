use myc_config::{
    optional_config::OptionalConfig, secret_resolver::SecretResolver,
};
use myc_core::domain::dtos::{
    callback::{Callback, ExecutionMode},
    health_check_info::HealthStatus,
    http::Protocol,
    route::Route,
    service::{Service, ServiceHost, ServiceSecret, ServiceType},
    service_notification_config::ServiceNotificationConfig,
};
use myc_core::models::ServiceNotificationsConfig;

use super::webapp_config::WebappConfig;
use mycelium_base::utils::errors::{creation_err, MappedErrors};
use serde::{
    de::{MapAccess, SeqAccess, Visitor},
    Deserialize, Deserializer,
};
use std::{
    collections::HashSet,
    env, fmt,
    path::{Path, PathBuf},
};
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

impl Default for LogFormat {
    fn default() -> Self {
        LogFormat::Ansi
    }
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
    #[serde(default = "default_logging_level")]
    pub level: String,
    #[serde(default)]
    pub format: LogFormat,
    #[serde(default = "default_logging_target")]
    pub target: Option<LoggingTarget>,
}

fn default_logging_level() -> String {
    "mycelium_base=info,myc_api=info,myc_config=info,myc_core=info,\
     myc_http_tools=info,actix_web=info,myc_notifier=info,\
     myc_diesel_sqlite=info,myc_moka_cache=info"
        .to_string()
}

fn default_logging_target() -> Option<LoggingTarget> {
    Some(LoggingTarget::Stdout)
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
    /// The time to live for the profile cache. Kept short: nothing evicts a
    /// cached profile when an invitation, tenant ownership or account change
    /// alters it, so this bounds how long a user sees the old profile.
    pub profile_ttl: Option<u64>,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            jwks_ttl: Some(60 * 60 * 12), // 12 hours
            email_ttl: Some(60 * 10),     // 10 minutes
            profile_ttl: Some(60),        // 1 minute
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
    #[serde(default)]
    notifications: Option<ServiceNotificationConfig>,
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
        notifications: intermediate.notifications,
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

        // Handle inline array format: callbacks = [] or callbacks = [{ ... }]
        fn visit_seq<S>(self, mut seq: S) -> Result<Self::Value, S::Error>
        where
            S: SeqAccess<'de>,
        {
            let mut all_callbacks = Vec::new();
            while let Some(callback) = seq.next_element::<Callback>()? {
                all_callbacks.push(callback);
            }
            if all_callbacks.is_empty() {
                return Ok(None);
            }
            Ok(Some(all_callbacks))
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
            deserializer.deserialize_any(CallbacksVisitor)
        }
    }

    deserializer.deserialize_option(CallbacksVisitor)
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiConfig {
    #[serde(default = "default_service_ip")]
    pub service_ip: String,
    #[serde(default = "default_service_port")]
    pub service_port: u16,
    #[serde(default)]
    pub allowed_origins: Vec<String>,
    #[serde(default = "default_service_workers")]
    pub service_workers: i32,
    #[serde(default = "default_gateway_timeout")]
    pub gateway_timeout: u64,
    pub logging: LoggingConfig,
    #[serde(default)]
    pub tls: OptionalConfig<TlsConfig>,
    #[serde(default)]
    pub cache: CacheConfig,
    pub health_check_interval: Option<u64>,
    pub max_retry_count: Option<u32>,
    pub max_error_instances: Option<u32>,

    #[serde(
        default,
        deserialize_with = "deserialize_callbacks",
        skip_serializing_if = "Option::is_none"
    )]
    pub callbacks: Option<Vec<Callback>>,

    #[serde(default)]
    pub callback_execution_mode: ExecutionMode,

    #[serde(default, deserialize_with = "deserialize_services")]
    pub services: Vec<Service>,

    /// OpenRPC discovery: development server URL (e.g. http://localhost:8080/_adm/rpc).
    /// Overridable by env MYCELIUM_OPENRPC_DEV_URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub openrpc_dev_url: Option<String>,

    /// OpenRPC discovery: production (or public) server URL (e.g. https://api.mycelium.io/_adm/rpc).
    /// Overridable by env MYCELIUM_OPENRPC_PROD_URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub openrpc_prod_url: Option<String>,

    /// The webapp build served under `/_app/` (`webapp` feature)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub webapp: Option<WebappConfig>,
}

fn default_service_ip() -> String {
    "0.0.0.0".to_string()
}

fn default_service_port() -> u16 {
    8080
}

fn default_service_workers() -> i32 {
    1
}

fn default_gateway_timeout() -> u64 {
    60
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TmpConfig {
    api: ApiConfig,

    /// Only the `[core.serviceNotifications]` block is read here, to
    /// cross-check it against the services that enable notifications.
    #[serde(default)]
    core: TmpCoreConfig,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TmpCoreConfig {
    #[serde(default)]
    service_notifications: ServiceNotificationsConfig,
}

/// Cross-service checks for the `notifications` blocks
///
/// Runs after `deserialize_services`, since each block is validated alone at
/// parse time. Sender tags must be unique, and any enabled service requires a
/// `templatesDir` isolated from the gateway's own `TEMPLATES_DIR`.
fn validate_service_notifications(
    services: &[Service],
    notifications_config: &ServiceNotificationsConfig,
    gateway_templates_dir: &Path,
) -> Result<(), String> {
    if !validate_unique_sender_tags(services)? {
        return Ok(());
    }

    let Some(templates_dir) = notifications_config.templates_dir.as_ref()
    else {
        return Err("core.serviceNotifications.templatesDir is required \
                    when a service enables notifications"
            .to_string());
    };

    validate_isolated_templates_dir(templates_dir, gateway_templates_dir)
}

/// Reject duplicate sender tags; return whether any service is enabled.
fn validate_unique_sender_tags(services: &[Service]) -> Result<bool, String> {
    let mut sender_tags = HashSet::new();

    for notifications in services
        .iter()
        .filter_map(|service| service.notifications.as_ref())
    {
        if !sender_tags.insert(notifications.sender_tag.as_str()) {
            return Err(format!(
                "Duplicate notifications.senderTag: {}",
                notifications.sender_tag
            ));
        }
    }

    Ok(!sender_tags.is_empty())
}

/// The service templates root and `TEMPLATES_DIR` must be disjoint
///
/// Neither may equal or contain the other: the service Tera instance loads
/// `templatesDir/**/*`, so an ancestor would also load the gateway's own
/// lifecycle templates, and a descendant would be loaded by the gateway's.
fn validate_isolated_templates_dir(
    templates_dir: &Path,
    gateway_templates_dir: &Path,
) -> Result<(), String> {
    let service_root = resolve_dir(templates_dir);
    let gateway_root = resolve_dir(gateway_templates_dir);

    if service_root.starts_with(&gateway_root)
        || gateway_root.starts_with(&service_root)
    {
        return Err(format!(
            "core.serviceNotifications.templatesDir ({}) must not equal, \
             contain or sit inside TEMPLATES_DIR ({})",
            templates_dir.display(),
            gateway_templates_dir.display()
        ));
    }

    Ok(())
}

/// Resolve symlinks when the directory exists, otherwise make it absolute.
fn resolve_dir(dir: &Path) -> PathBuf {
    dir.canonicalize()
        .or_else(|_| std::path::absolute(dir))
        .unwrap_or_else(|_| dir.to_path_buf())
}

/// The gateway templates root, resolved as in `myc_core::settings`.
fn gateway_templates_dir() -> PathBuf {
    PathBuf::from(
        env::var("TEMPLATES_DIR").unwrap_or_else(|_| "templates".to_string()),
    )
}

/// Pre-process TOML content to transform [[service-name]] into
/// [[api.services.service-name]] when inside [api.services] context
/// and [[callback]] into [[api.callbacks.callback]] when inside [api.callbacks] context
fn preprocess_toml_services(content: &str) -> String {
    let mut result = String::new();
    let mut in_services_context = false;
    let mut in_callbacks_context = false;
    let mut seen_services = HashSet::<String>::new();

    for line in content.lines() {
        let trimmed = line.trim();

        // Transform [service-name.notifications] into
        // [api.services.service-name.notifications], which TOML attaches to
        // the last [[api.services.service-name]] entry. Only the notifications
        // sub-table of an already declared service is rewritten, so unrelated
        // top-level tables keep closing the services context below.
        if in_services_context {
            if let Some(new_line) =
                rewrite_service_notifications_table(line, &seen_services)
            {
                result.push_str(&new_line);
                result.push('\n');
                continue;
            }
        }

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
                    seen_services.insert(
                        service_name
                            .split('.')
                            .next()
                            .unwrap_or_default()
                            .to_string(),
                    );
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

/// Rewrite `[service-name.notifications]` when `service-name` was declared
fn rewrite_service_notifications_table(
    line: &str,
    seen_services: &HashSet<String>,
) -> Option<String> {
    let service_name = line
        .trim()
        .strip_prefix('[')?
        .strip_suffix(']')?
        .strip_suffix(".notifications")?;

    if !seen_services.contains(service_name) {
        return None;
    }

    Some(line.replacen(
        &format!("[{service_name}."),
        &format!("[api.services.{service_name}."),
        1,
    ))
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

        validate_service_notifications(
            &config.api.services,
            &config.core.service_notifications,
            &gateway_templates_dir(),
        )
        .map_err(|err| {
            creation_err(format!("Invalid services notifications: {err}"))
        })?;

        Ok(config.api)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_config_tuning_fields_default_when_absent() {
        let toml = r#"
            [logging]
        "#;
        let config: ApiConfig = toml::from_str(toml).unwrap();

        assert_eq!(config.service_ip, "0.0.0.0");
        assert_eq!(config.service_port, 8080);
        assert_eq!(config.service_workers, 1);
        assert_eq!(config.gateway_timeout, 60);
        assert_eq!(config.allowed_origins, Vec::<String>::new());
        assert!(matches!(config.tls, OptionalConfig::Disabled));
        assert_eq!(config.cache.jwks_ttl, CacheConfig::default().jwks_ttl);
        assert_eq!(config.cache.email_ttl, CacheConfig::default().email_ttl);
        assert_eq!(
            config.cache.profile_ttl,
            CacheConfig::default().profile_ttl
        );
    }

    fn parse_services(toml_content: &str) -> Result<ApiConfig, String> {
        let preprocessed = preprocess_toml_services(toml_content);
        toml::from_str::<TmpConfig>(&preprocessed)
            .map(|config| config.api)
            .map_err(|e| e.to_string())
    }

    #[test]
    fn route_retry_policy_parses_from_the_service_path_table() {
        let config = parse_services(
            r#"
            [api]
            [api.logging]

            [api.services]

            [[inventory]]
            host = "localhost:8088"
            healthCheckPath = "/health"

            [[inventory.path]]
            group = "public"
            path = "/items/*"
            methods = ["GET"]
            retry = { maxAttempts = 4, baseDelayMs = 50, maxDelayMs = 500, retryOnStatus = [503] }

            [[inventory.path]]
            group = "public"
            path = "/orders/*"
            methods = ["POST"]
            "#,
        )
        .unwrap();

        let routes = &config.services[0].routes;
        let retry = routes[0].retry.as_ref().expect("retry parsed");
        assert_eq!(retry.max_attempts, 4);
        assert_eq!(retry.base_delay_ms, 50);
        assert_eq!(retry.max_delay_ms, 500);
        assert_eq!(retry.retry_on_status, vec![503]);
        assert!(routes[1].retry.is_none());
    }

    #[test]
    fn invalid_route_retry_policy_fails_config_load() {
        let error = parse_services(
            r#"
            [api]
            [api.logging]

            [api.services]

            [[inventory]]
            host = "localhost:8088"
            healthCheckPath = "/health"

            [[inventory.path]]
            group = "public"
            path = "/items/*"
            methods = ["GET"]
            retry = { maxAttempts = 50 }
            "#,
        )
        .unwrap_err();

        assert!(error.contains("maxAttempts"), "{error}");
    }

    const NOTIFIED_SERVICE: &str = r#"
            [api]
            [api.logging]

            [api.services]

            [[service-01]]
            host = "localhost:8088"
            healthCheckPath = "/health"

            [service-01.notifications]
            senderTag = "service-01"
            displayName = "Service 01"
            issuerRoles = ["service-01-admin"]
            recipientPolicy = "owner"
            templates = ["invoice-ready"]

            [[service-01.path]]
            group = "public"
            path = "/items/*"
            methods = ["GET"]
            "#;

    fn load_from_file(content: &str) -> Result<ApiConfig, MappedErrors> {
        let file = env::temp_dir()
            .join(format!("myc-api-config-{}.toml", Uuid::new_v4()));

        std::fs::write(&file, content).unwrap();
        let config = ApiConfig::from_default_config_file(file.clone());
        std::fs::remove_file(&file).unwrap();

        config
    }

    fn with_core_block(api: &str, templates_dir: Option<&Path>) -> String {
        let Some(templates_dir) = templates_dir else {
            return api.to_string();
        };

        format!(
            "[core.serviceNotifications]\ntemplatesDir = {:?}\n{api}",
            templates_dir.display().to_string()
        )
    }

    fn isolated_templates_dir() -> PathBuf {
        env::temp_dir()
            .join(format!("myc-service-templates-{}", Uuid::new_v4()))
    }

    #[test]
    fn service_without_notifications_block_parses_as_before() {
        let config = load_from_file(
            r#"
            [api]
            [api.logging]

            [api.services]

            [[inventory]]
            host = "localhost:8088"
            healthCheckPath = "/health"
            "#,
        )
        .unwrap();

        assert!(config.services[0].notifications.is_none());
    }

    #[test]
    fn notifications_table_attaches_to_its_service() {
        let config = parse_services(NOTIFIED_SERVICE).unwrap();

        let service = &config.services[0];
        let notifications = service
            .notifications
            .as_ref()
            .expect("notifications parsed");

        assert_eq!(service.name, "service-01");
        assert_eq!(notifications.sender_tag, "service-01");
        assert_eq!(service.routes.len(), 1);
    }

    #[test]
    fn notifications_inline_table_parses() {
        let config = parse_services(
            r#"
            [api]
            [api.logging]

            [api.services]

            [[service-01]]
            host = "localhost:8088"
            healthCheckPath = "/health"
            notifications = { senderTag = "service-01", displayName = "Service 01", issuerRoles = ["admin"], recipientPolicy = "any", templates = ["a"] }
            "#,
        )
        .unwrap();

        assert!(config.services[0].notifications.is_some());
    }

    #[test]
    fn notifications_table_of_unknown_service_is_not_rewritten() {
        let mut seen = HashSet::new();
        seen.insert("service-01".to_string());

        assert_eq!(
            rewrite_service_notifications_table(
                "[service-01.notifications]",
                &seen
            ),
            Some("[api.services.service-01.notifications]".to_string())
        );
        assert!(rewrite_service_notifications_table(
            "[other.notifications]",
            &seen
        )
        .is_none());
        assert!(rewrite_service_notifications_table("[core.webhook]", &seen)
            .is_none());
    }

    #[test]
    fn invalid_notifications_block_fails_config_load() {
        let error = parse_services(
            &NOTIFIED_SERVICE.replace(r#"["service-01-admin"]"#, "[]"),
        )
        .unwrap_err();

        assert!(error.contains("issuerRoles"), "{error}");
    }

    #[test]
    fn enabled_service_with_isolated_templates_dir_loads() {
        let templates_dir = isolated_templates_dir();
        let config = load_from_file(&with_core_block(
            NOTIFIED_SERVICE,
            Some(&templates_dir),
        ))
        .unwrap();

        assert!(config.services[0].notifications.is_some());
    }

    #[test]
    fn duplicate_sender_tags_fail_config_load() {
        let templates_dir = isolated_templates_dir();
        let duplicated = format!(
            "{NOTIFIED_SERVICE}{}",
            r#"
            [[service-02]]
            host = "localhost:8089"
            healthCheckPath = "/health"

            [service-02.notifications]
            senderTag = "service-01"
            displayName = "Service 02"
            issuerRoles = ["service-01-admin"]
            recipientPolicy = "owner"
            templates = ["invoice-ready"]
            "#
        );

        let error =
            load_from_file(&with_core_block(&duplicated, Some(&templates_dir)))
                .unwrap_err();

        assert!(error.to_string().contains("Duplicate"), "{error}");
    }

    #[test]
    fn enabled_service_without_templates_dir_fails_config_load() {
        let error = load_from_file(NOTIFIED_SERVICE).unwrap_err();

        assert!(error.to_string().contains("templatesDir"), "{error}");
    }

    #[test]
    fn templates_dir_overlapping_gateway_templates_fails_config_load() {
        let gateway_dir = gateway_templates_dir();

        for templates_dir in [gateway_dir.clone(), gateway_dir.join("services")]
        {
            let error = load_from_file(&with_core_block(
                NOTIFIED_SERVICE,
                Some(&templates_dir),
            ))
            .unwrap_err();

            assert!(error.to_string().contains("TEMPLATES_DIR"), "{error}");
        }
    }

    #[test]
    fn templates_dir_overlap_check_uses_given_gateway_dir() {
        let services = parse_services(NOTIFIED_SERVICE).unwrap().services;
        let gateway_dir = PathBuf::from("/srv/myc/templates");
        let config_for = |dir: &str| ServiceNotificationsConfig {
            templates_dir: Some(PathBuf::from(dir)),
            ..Default::default()
        };

        for overlapping in [
            "/srv/myc/templates",
            "/srv/myc/templates/svc",
            "/srv/myc",
            "/",
        ] {
            assert!(validate_service_notifications(
                &services,
                &config_for(overlapping),
                &gateway_dir
            )
            .is_err());
        }

        for isolated in ["/srv/myc/service-templates", "/srv/other"] {
            assert!(validate_service_notifications(
                &services,
                &config_for(isolated),
                &gateway_dir
            )
            .is_ok());
        }
    }

    #[test]
    fn templates_dir_not_required_without_enabled_services() {
        assert!(validate_service_notifications(
            &[],
            &ServiceNotificationsConfig::default(),
            Path::new("templates"),
        )
        .is_ok());
    }

    #[test]
    fn logging_config_defaults_when_fields_absent() {
        let config: LoggingConfig = toml::from_str("").unwrap();

        assert_eq!(
            config.level,
            "mycelium_base=info,myc_api=info,myc_config=info,myc_core=info,\
             myc_http_tools=info,actix_web=info,myc_notifier=info,\
             myc_diesel_sqlite=info,myc_moka_cache=info"
        );
        assert!(matches!(config.format, LogFormat::Ansi));
        assert!(matches!(config.target, Some(LoggingTarget::Stdout)));
    }
}
