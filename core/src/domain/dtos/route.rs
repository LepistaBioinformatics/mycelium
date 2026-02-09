use super::{
    http::HttpMethod, http_secret::HttpSecret, security_group::SecurityGroup,
    service::Service,
};

use http::{uri::PathAndQuery, Uri};
use mycelium_base::{
    dtos::Parent,
    utils::errors::{dto_err, execution_err, MappedErrors},
};
use serde::{Deserialize, Serialize};
use utoipa::{ToResponse, ToSchema};
use uuid::Uuid;

fn default_service() -> Parent<Service, Uuid> {
    Parent::Id(Uuid::nil())
}

#[derive(
    Debug, Clone, Deserialize, Serialize, ToSchema, ToResponse, PartialEq, Eq,
)]
#[serde(rename_all = "camelCase")]
pub struct Route {
    /// The route id
    pub id: Option<Uuid>,

    /// The route service
    #[serde(default = "default_service")]
    pub service: Parent<Service, Uuid>,

    /// The route name
    #[serde(alias = "group")]
    pub security_group: SecurityGroup,

    /// The route description
    pub methods: Vec<HttpMethod>,

    /// The route url
    pub path: String,

    /// The route secret name if it exists
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret_name: Option<String>,

    /// The route without tls
    ///
    /// This field should be evaluated if the route should request a secret to
    /// be send to the downstream service, if the route is not secure.
    ///
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accept_insecure_routing: Option<bool>,

    /// Callbacks
    ///
    /// A vector of callback names to execute.
    ///
    /// Example:
    ///
    /// ```json
    /// ["my_callback", "my_callback_2"]
    /// ```
    ///
    #[serde(skip_serializing_if = "Option::is_none")]
    pub callbacks: Option<Vec<String>>,
}

impl Route {
    pub fn new(
        id: Option<Uuid>,
        service: Service,
        group: SecurityGroup,
        methods: Vec<HttpMethod>,
        path: String,
        secret_name: Option<String>,
        accept_insecure_routing: Option<bool>,
        callbacks: Option<Vec<String>>,
    ) -> Self {
        Self {
            id: match id {
                Some(id) => Some(id),
                None => Some(Uuid::new_v3(
                    &Uuid::NAMESPACE_DNS,
                    format!(
                        "{service_name}-{path}-{methods}",
                        service_name = service.name,
                        path = path,
                        methods = methods
                            .iter()
                            .map(|m| m.to_string())
                            .collect::<Vec<String>>()
                            .join("-")
                    )
                    .as_bytes(),
                )),
            },
            service: Parent::Record(service),
            security_group: group,
            methods,
            path,
            secret_name,
            accept_insecure_routing,
            callbacks,
        }
    }

    /// Check if a method is allowed
    ///
    /// Rules:
    /// - If the methods list includes None, nothing is allowed
    /// - If the methods list includes All, all methods are allowed
    /// - If the methods list includes Read, all read methods are allowed, like
    ///   GET, HEAD, OPTIONS, TRACE.
    /// - If the methods list includes Write, all write methods are allowed,
    ///   like POST, PUT, PATCH, DELETE.
    ///
    /// Users can declared a combination of read and write methods to simplify
    /// the declaration, like:
    /// - ["GET", "POST"]
    /// - ["READ", "POST"]
    /// - ["WRITE", "GET"]
    ///
    pub async fn allow_method(&self, method: HttpMethod) -> Option<HttpMethod> {
        //
        // Check if the methods list includes None. If so, return None
        //
        if self.methods.contains(&HttpMethod::None) {
            return None;
        }

        //
        // Check if the methods list includes All. If so, return the method
        //
        if self.methods.contains(&HttpMethod::All) {
            return Some(method);
        }

        //
        // Check if the method is a read method and the methods list includes
        // Read. If so, return the method
        //
        if method.is_read_method() && self.methods.contains(&HttpMethod::Read) {
            return Some(method);
        }

        //
        // Check if the method is a write method and the methods list includes
        // Write. If so, return the method
        //
        if method.is_write_method() && self.methods.contains(&HttpMethod::Write)
        {
            return Some(method);
        }

        //
        // Check for a specific method in the methods list
        //
        match self.methods.contains(&method) {
            true => Some(method),
            false => None,
        }
    }

    /// Build a actix_web::http::Uri from itself.
    pub async fn build_uri(&self) -> Result<Uri, MappedErrors> {
        let service = match self.service {
            Parent::Record(ref service) => service,
            Parent::Id(_) => {
                return execution_err(
                    "Unexpected error on build URI: service not found",
                )
                .as_error()
            }
        };

        let host = service.to_owned().host.choose_host();
        let path_parts = host.split("/").collect::<Vec<&str>>();
        let domain = path_parts[0];

        match Uri::builder()
            .scheme(service.protocol.to_string().as_str())
            .authority(domain)
            .path_and_query(self.path.as_str())
            .build()
        {
            Err(err) => {
                execution_err(format!("Unexpected error on build URI: {}", err))
                    .as_error()
            }
            Ok(res) => Ok(res),
        }
    }

    /// Extend a Uri from a base Uri (deprecated).
    ///
    /// This function extends a Uri from a base Uri. It is deprecated because it
    /// is not used anymore.
    ///
    #[deprecated(since = "8.2.2-beta.2", note = "Use build_uri instead")]
    pub async fn extend_uri(
        uri: Uri,
        extension: PathAndQuery,
    ) -> Result<Uri, MappedErrors> {
        // Build the extended path
        let path = uri.path().to_owned() + extension.path();

        // Build parameters vector
        let params: &str = &vec![uri.query(), extension.query()]
            .into_iter()
            .filter_map(|p| p.map(|res| res))
            .collect::<Vec<&str>>()
            .join("&")
            .to_owned();

        // Join path with params if it exists
        let path_and_query = match params.chars().count() {
            0 => path,
            _ => path + "?" + params,
        };

        match Uri::builder()
            .scheme(uri.scheme().unwrap().to_string().as_str())
            .authority(uri.authority().unwrap().as_str())
            .path_and_query(path_and_query)
            .build()
        {
            Err(err) => {
                execution_err(format!("Unexpected error on build URI: {}", err))
                    .as_error()
            }
            Ok(res) => Ok(res),
        }
    }

    pub async fn solve_secret(
        &self,
    ) -> Result<Option<HttpSecret>, MappedErrors> {
        if let Some(secret_name) = &self.secret_name {
            match self.service.to_owned() {
                Parent::Id(_) => {
                    return dto_err(format!(
                        "Unable to solve secret (invalid service object): {secret_name}",
                        secret_name = secret_name
                    ))
                    .as_error();
                }
                Parent::Record(service) => match service.secrets {
                    Some(secret) => {
                        match secret.iter().find(|s| s.name == *secret_name) {
                            Some(secret) => {
                                let secret_resolver = &secret.secret;
                                let secret = secret_resolver
                                    .async_get_or_error()
                                    .await?;

                                return Ok(Some(secret));
                            }
                            None => {
                                return dto_err(format!(
                                    "Unable to solve secret (secret not available): {secret_name}",
                                    secret_name = secret_name
                                ))
                                .as_error();
                            }
                        }
                    }
                    None => {
                        return dto_err(format!(
                            "Unable to solve secret (service secrets is empty): {secret_name}",
                            secret_name = secret_name
                        ))
                        .as_error();
                    }
                },
            };
        }

        Ok(None)
    }

    pub fn get_service_id(&self) -> Uuid {
        match self.service.to_owned() {
            Parent::Id(id) => id,
            Parent::Record(record) => record.id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::dtos::{
        health_check_info::HealthStatus,
        http::Protocol,
        service::{ServiceHost, ServiceSecret},
    };
    use mycelium_base::dtos::Parent;

    fn create_test_route(methods: Vec<HttpMethod>) -> Route {
        let service = Service {
            id: Uuid::new_v4(),
            name: "test-service".to_string(),
            host: ServiceHost::Host("localhost:8080".to_string()),
            protocol: Protocol::Http,
            routes: vec![],
            health_status: HealthStatus::Unknown,
            health_check_path: "/health".to_string(),
            discoverable: None,
            service_type: None,
            is_context_api: None,
            capabilities: None,
            description: None,
            openapi_path: None,
            secrets: None,
            allowed_sources: None,
            proxy_address: None,
        };

        Route {
            id: Some(Uuid::new_v4()),
            service: Parent::Record(service),
            security_group: SecurityGroup::Public,
            methods,
            path: "/test".to_string(),
            secret_name: None,
            accept_insecure_routing: None,
            callbacks: None,
        }
    }

    #[tokio::test]
    async fn test_allow_method_with_none_returns_none() {
        let route = create_test_route(vec![HttpMethod::None]);
        let result = route.allow_method(HttpMethod::Get).await;
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_allow_method_with_none_and_other_methods_returns_none() {
        let route = create_test_route(vec![HttpMethod::None, HttpMethod::Get]);
        let result = route.allow_method(HttpMethod::Get).await;
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_allow_method_with_all_returns_method() {
        let route = create_test_route(vec![HttpMethod::All]);
        let result = route.allow_method(HttpMethod::Get).await;
        assert_eq!(result, Some(HttpMethod::Get));
    }

    #[tokio::test]
    async fn test_allow_method_with_all_returns_any_method() {
        let route = create_test_route(vec![HttpMethod::All]);
        assert_eq!(
            route.allow_method(HttpMethod::Post).await,
            Some(HttpMethod::Post)
        );
        assert_eq!(
            route.allow_method(HttpMethod::Put).await,
            Some(HttpMethod::Put)
        );
        assert_eq!(
            route.allow_method(HttpMethod::Delete).await,
            Some(HttpMethod::Delete)
        );
        assert_eq!(
            route.allow_method(HttpMethod::Connect).await,
            Some(HttpMethod::Connect)
        );
    }

    #[tokio::test]
    async fn test_allow_method_with_read_allows_get() {
        let route = create_test_route(vec![HttpMethod::Read]);
        let result = route.allow_method(HttpMethod::Get).await;
        assert_eq!(result, Some(HttpMethod::Get));
    }

    #[tokio::test]
    async fn test_allow_method_with_read_allows_head() {
        let route = create_test_route(vec![HttpMethod::Read]);
        let result = route.allow_method(HttpMethod::Head).await;
        assert_eq!(result, Some(HttpMethod::Head));
    }

    #[tokio::test]
    async fn test_allow_method_with_read_allows_options() {
        let route = create_test_route(vec![HttpMethod::Read]);
        let result = route.allow_method(HttpMethod::Options).await;
        assert_eq!(result, Some(HttpMethod::Options));
    }

    #[tokio::test]
    async fn test_allow_method_with_read_allows_trace() {
        let route = create_test_route(vec![HttpMethod::Read]);
        let result = route.allow_method(HttpMethod::Trace).await;
        assert_eq!(result, Some(HttpMethod::Trace));
    }

    #[tokio::test]
    async fn test_allow_method_with_read_denies_post() {
        let route = create_test_route(vec![HttpMethod::Read]);
        let result = route.allow_method(HttpMethod::Post).await;
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_allow_method_with_read_denies_write_methods() {
        let route = create_test_route(vec![HttpMethod::Read]);
        assert_eq!(route.allow_method(HttpMethod::Post).await, None);
        assert_eq!(route.allow_method(HttpMethod::Put).await, None);
        assert_eq!(route.allow_method(HttpMethod::Patch).await, None);
        assert_eq!(route.allow_method(HttpMethod::Delete).await, None);
    }

    #[tokio::test]
    async fn test_allow_method_with_write_allows_post() {
        let route = create_test_route(vec![HttpMethod::Write]);
        let result = route.allow_method(HttpMethod::Post).await;
        assert_eq!(result, Some(HttpMethod::Post));
    }

    #[tokio::test]
    async fn test_allow_method_with_write_allows_put() {
        let route = create_test_route(vec![HttpMethod::Write]);
        let result = route.allow_method(HttpMethod::Put).await;
        assert_eq!(result, Some(HttpMethod::Put));
    }

    #[tokio::test]
    async fn test_allow_method_with_write_allows_patch() {
        let route = create_test_route(vec![HttpMethod::Write]);
        let result = route.allow_method(HttpMethod::Patch).await;
        assert_eq!(result, Some(HttpMethod::Patch));
    }

    #[tokio::test]
    async fn test_allow_method_with_write_allows_delete() {
        let route = create_test_route(vec![HttpMethod::Write]);
        let result = route.allow_method(HttpMethod::Delete).await;
        assert_eq!(result, Some(HttpMethod::Delete));
    }

    #[tokio::test]
    async fn test_allow_method_with_write_denies_get() {
        let route = create_test_route(vec![HttpMethod::Write]);
        let result = route.allow_method(HttpMethod::Get).await;
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_allow_method_with_write_denies_read_methods() {
        let route = create_test_route(vec![HttpMethod::Write]);
        assert_eq!(route.allow_method(HttpMethod::Get).await, None);
        assert_eq!(route.allow_method(HttpMethod::Head).await, None);
        assert_eq!(route.allow_method(HttpMethod::Options).await, None);
        assert_eq!(route.allow_method(HttpMethod::Trace).await, None);
    }

    #[tokio::test]
    async fn test_allow_method_with_specific_method_allows_it() {
        let route = create_test_route(vec![HttpMethod::Get]);
        let result = route.allow_method(HttpMethod::Get).await;
        assert_eq!(result, Some(HttpMethod::Get));
    }

    #[tokio::test]
    async fn test_allow_method_with_specific_method_denies_others() {
        let route = create_test_route(vec![HttpMethod::Get]);
        assert_eq!(route.allow_method(HttpMethod::Post).await, None);
        assert_eq!(route.allow_method(HttpMethod::Put).await, None);
        assert_eq!(route.allow_method(HttpMethod::Delete).await, None);
    }

    #[tokio::test]
    async fn test_allow_method_with_multiple_specific_methods() {
        let route = create_test_route(vec![HttpMethod::Get, HttpMethod::Post]);
        assert_eq!(
            route.allow_method(HttpMethod::Get).await,
            Some(HttpMethod::Get)
        );
        assert_eq!(
            route.allow_method(HttpMethod::Post).await,
            Some(HttpMethod::Post)
        );
        assert_eq!(route.allow_method(HttpMethod::Put).await, None);
    }

    #[tokio::test]
    async fn test_allow_method_with_read_and_specific_method() {
        let route = create_test_route(vec![HttpMethod::Read, HttpMethod::Post]);
        // Read methods should be allowed
        assert_eq!(
            route.allow_method(HttpMethod::Get).await,
            Some(HttpMethod::Get)
        );
        assert_eq!(
            route.allow_method(HttpMethod::Head).await,
            Some(HttpMethod::Head)
        );
        // Specific write method should be allowed
        assert_eq!(
            route.allow_method(HttpMethod::Post).await,
            Some(HttpMethod::Post)
        );
        // Other write methods should not be allowed
        assert_eq!(route.allow_method(HttpMethod::Put).await, None);
    }

    #[tokio::test]
    async fn test_allow_method_with_write_and_specific_method() {
        let route = create_test_route(vec![HttpMethod::Write, HttpMethod::Get]);
        // Write methods should be allowed
        assert_eq!(
            route.allow_method(HttpMethod::Post).await,
            Some(HttpMethod::Post)
        );
        assert_eq!(
            route.allow_method(HttpMethod::Put).await,
            Some(HttpMethod::Put)
        );
        // Specific read method should be allowed
        assert_eq!(
            route.allow_method(HttpMethod::Get).await,
            Some(HttpMethod::Get)
        );
        // Other read methods should not be allowed
        assert_eq!(route.allow_method(HttpMethod::Head).await, None);
    }

    #[tokio::test]
    async fn test_allow_method_with_connect_not_read_or_write() {
        let route =
            create_test_route(vec![HttpMethod::Read, HttpMethod::Write]);
        // Connect is neither read nor write, so it should not be allowed
        assert_eq!(route.allow_method(HttpMethod::Connect).await, None);
    }

    #[tokio::test]
    async fn test_allow_method_with_connect_specific() {
        let route = create_test_route(vec![HttpMethod::Connect]);
        // Connect should be allowed if explicitly specified
        assert_eq!(
            route.allow_method(HttpMethod::Connect).await,
            Some(HttpMethod::Connect)
        );
    }

    #[tokio::test]
    async fn test_allow_method_empty_methods_list() {
        let route = create_test_route(vec![]);
        assert_eq!(route.allow_method(HttpMethod::Get).await, None);
        assert_eq!(route.allow_method(HttpMethod::Post).await, None);
    }

    // ? -----------------------------------------------------------------------
    // ? Tests for solve_secret method
    // ? -----------------------------------------------------------------------

    fn create_test_route_with_secret_name(
        secret_name: Option<String>,
        service: Parent<Service, Uuid>,
    ) -> Route {
        Route {
            id: Some(Uuid::new_v4()),
            service,
            security_group: SecurityGroup::Public,
            methods: vec![HttpMethod::Get],
            path: "/test".to_string(),
            secret_name,
            accept_insecure_routing: None,
            callbacks: None,
        }
    }

    fn create_test_service_with_secrets(
        secrets: Option<Vec<ServiceSecret>>,
    ) -> Service {
        Service {
            id: Uuid::new_v4(),
            name: "test-service".to_string(),
            host: ServiceHost::Host("localhost:8080".to_string()),
            protocol: Protocol::Http,
            routes: vec![],
            health_status: HealthStatus::Unknown,
            health_check_path: "/health".to_string(),
            discoverable: None,
            service_type: None,
            is_context_api: None,
            capabilities: None,
            description: None,
            openapi_path: None,
            secrets,
            allowed_sources: None,
            proxy_address: None,
        }
    }

    #[tokio::test]
    async fn test_solve_secret_without_secret_name_returns_none() {
        let service = create_test_service_with_secrets(None);
        let route =
            create_test_route_with_secret_name(None, Parent::Record(service));
        let result = route.solve_secret().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);
    }

    #[tokio::test]
    async fn test_solve_secret_with_service_id_returns_error() {
        let route = create_test_route_with_secret_name(
            Some("test-secret".to_string()),
            Parent::Id(Uuid::new_v4()),
        );
        let result = route.solve_secret().await;
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error
            .to_string()
            .contains("Unable to solve secret (invalid service object)"));
        assert!(error.to_string().contains("test-secret"));
    }

    #[tokio::test]
    async fn test_solve_secret_with_empty_secrets_returns_error() {
        let service = create_test_service_with_secrets(None);
        let route = create_test_route_with_secret_name(
            Some("test-secret".to_string()),
            Parent::Record(service),
        );
        let result = route.solve_secret().await;
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error
            .to_string()
            .contains("Unable to solve secret (service secrets is empty)"));
        assert!(error.to_string().contains("test-secret"));
    }

    #[tokio::test]
    async fn test_solve_secret_with_secret_not_found_returns_error() {
        use myc_config::secret_resolver::SecretResolver;

        let secret = HttpSecret::AuthorizationHeader {
            header_name: Some("Authorization".to_string()),
            prefix: Some("Bearer".to_string()),
            token: "token123".to_string(),
        };

        let service_secrets = vec![ServiceSecret {
            name: "other-secret".to_string(),
            secret: SecretResolver::Value(secret),
        }];

        let service = create_test_service_with_secrets(Some(service_secrets));
        let route = create_test_route_with_secret_name(
            Some("test-secret".to_string()),
            Parent::Record(service),
        );
        let result = route.solve_secret().await;
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error
            .to_string()
            .contains("Unable to solve secret (secret not available)"));
        assert!(error.to_string().contains("test-secret"));
    }

    #[tokio::test]
    async fn test_solve_secret_with_secret_found_returns_secret() {
        use myc_config::secret_resolver::SecretResolver;

        let secret = HttpSecret::AuthorizationHeader {
            header_name: Some("Authorization".to_string()),
            prefix: Some("Bearer".to_string()),
            token: "token123".to_string(),
        };

        let service_secrets = vec![ServiceSecret {
            name: "test-secret".to_string(),
            secret: SecretResolver::Value(secret.clone()),
        }];

        let service = create_test_service_with_secrets(Some(service_secrets));
        let route = create_test_route_with_secret_name(
            Some("test-secret".to_string()),
            Parent::Record(service),
        );
        let result = route.solve_secret().await;
        assert!(result.is_ok());
        let resolved_secret = result.unwrap();
        assert!(resolved_secret.is_some());
        assert_eq!(resolved_secret.unwrap(), secret);
    }

    #[tokio::test]
    async fn test_solve_secret_with_multiple_secrets_finds_correct_one() {
        use myc_config::secret_resolver::SecretResolver;

        let secret1 = HttpSecret::AuthorizationHeader {
            header_name: Some("Authorization".to_string()),
            prefix: Some("Bearer".to_string()),
            token: "token1".to_string(),
        };

        let secret2 = HttpSecret::QueryParameter {
            name: "api_key".to_string(),
            token: "key123".to_string(),
        };

        let service_secrets = vec![
            ServiceSecret {
                name: "secret1".to_string(),
                secret: SecretResolver::Value(secret1),
            },
            ServiceSecret {
                name: "secret2".to_string(),
                secret: SecretResolver::Value(secret2.clone()),
            },
        ];

        let service = create_test_service_with_secrets(Some(service_secrets));
        let route = create_test_route_with_secret_name(
            Some("secret2".to_string()),
            Parent::Record(service),
        );
        let result = route.solve_secret().await;
        assert!(result.is_ok());
        let resolved_secret = result.unwrap();
        assert!(resolved_secret.is_some());
        assert_eq!(resolved_secret.unwrap(), secret2);
    }

    #[tokio::test]
    async fn test_solve_secret_with_query_parameter_secret() {
        use myc_config::secret_resolver::SecretResolver;

        let secret = HttpSecret::QueryParameter {
            name: "api_key".to_string(),
            token: "key123".to_string(),
        };

        let service_secrets = vec![ServiceSecret {
            name: "test-secret".to_string(),
            secret: SecretResolver::Value(secret.clone()),
        }];

        let service = create_test_service_with_secrets(Some(service_secrets));
        let route = create_test_route_with_secret_name(
            Some("test-secret".to_string()),
            Parent::Record(service),
        );
        let result = route.solve_secret().await;
        assert!(result.is_ok());
        let resolved_secret = result.unwrap();
        assert!(resolved_secret.is_some());
        assert_eq!(resolved_secret.unwrap(), secret);
    }
}
