use super::{
    http::{HttpMethod, Protocol},
    http_secret::HttpSecret,
    route_type::RouteType,
    service::Service,
};

use actix_web::http::{uri::PathAndQuery, Uri};
use mycelium_base::{
    dtos::Parent,
    utils::errors::{dto_err, execution_err, MappedErrors},
};
use serde::{Deserialize, Serialize};
use utoipa::{ToResponse, ToSchema};
use uuid::Uuid;

#[derive(
    Debug, Clone, Deserialize, Serialize, ToSchema, ToResponse, PartialEq, Eq,
)]
#[serde(rename_all = "camelCase")]
pub struct Route {
    /// The route id
    pub id: Option<Uuid>,

    /// The route service
    pub service: Parent<Service, Uuid>,

    /// The route name
    pub group: RouteType,

    /// The route description
    pub methods: Vec<HttpMethod>,

    /// The route url
    pub path: String,

    /// The route protocol
    pub protocol: Protocol,

    /// The route is active
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_sources: Option<Vec<String>>,

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
}

impl Route {
    pub fn new(
        id: Option<Uuid>,
        service: Service,
        group: RouteType,
        methods: Vec<HttpMethod>,
        path: String,
        protocol: Protocol,
        allowed_sources: Option<Vec<String>>,
        secret_name: Option<String>,
        accept_insecure_routing: Option<bool>,
    ) -> Self {
        Self {
            id: match id {
                Some(id) => Some(id),
                None => Some(Uuid::new_v3(
                    &Uuid::NAMESPACE_DNS,
                    format!(
                        "{service_name}-{protocol}-{path}-{methods}",
                        service_name = service.name,
                        protocol = protocol,
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
            group,
            methods,
            path,
            protocol,
            allowed_sources,
            secret_name,
            accept_insecure_routing,
        }
    }

    /// Check if a method is allowed.
    pub async fn allow_method(&self, method: HttpMethod) -> Option<HttpMethod> {
        if self.methods.contains(&HttpMethod::None) {
            return None;
        }

        if self.methods.contains(&HttpMethod::All) {
            return Some(method);
        }

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

        let host = service.to_owned().host;
        let path_parts = host.split("/").collect::<Vec<&str>>();
        let domain = path_parts[0];

        match Uri::builder()
            .scheme(self.protocol.to_string().as_str())
            .authority(domain)
            .path_and_query(self.path.as_str())
            .build()
        {
            Err(err) => {
                return execution_err(format!(
                    "Unexpected error on build URI: {}",
                    err
                ))
                .as_error()
            }
            Ok(res) => Ok(res),
        }
    }

    /// Extend a Uri from a base Uri.
    pub async fn extend_uri(
        uri: Uri,
        extension: PathAndQuery,
    ) -> Result<Uri, MappedErrors> {
        // Build the extended path
        let path = uri.path().to_owned() + extension.path();

        // Build parameters vector
        let params: &str = &vec![uri.query(), extension.query()]
            .into_iter()
            .filter_map(|p| match p {
                None => None,
                Some(res) => Some(res),
            })
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
                return execution_err(format!(
                    "Unexpected error on build URI: {}",
                    err
                ))
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
}
