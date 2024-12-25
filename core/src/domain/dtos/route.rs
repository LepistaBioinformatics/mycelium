use super::{
    http::{HttpMethod, Protocol},
    route_type::RouteType,
    service::Service,
};

use actix_web::http::{uri::PathAndQuery, Uri};
use mycelium_base::{
    dtos::Parent,
    utils::errors::{execution_err, MappedErrors},
};
use serde::{Deserialize, Serialize};
use utoipa::{ToResponse, ToSchema};
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, ToResponse)]
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
    pub downstream_url: String,

    /// The route protocol
    pub protocol: Protocol,

    /// The route is active
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_sources: Option<Vec<String>>,

    /// The route secret name if it exists
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret_name: Option<String>,
}

impl Route {
    pub fn new(
        id: Option<Uuid>,
        service: Service,
        group: RouteType,
        methods: Vec<HttpMethod>,
        downstream_url: String,
        protocol: Protocol,
        allowed_sources: Option<Vec<String>>,
        secret_name: Option<String>,
    ) -> Self {
        Self {
            id: match id {
                Some(id) => Some(id),
                None => Some(Uuid::new_v3(
                    &Uuid::NAMESPACE_DNS,
                    format!(
                        "{service_name}-{protocol}-{downstream_url}-{methods}",
                        service_name = service.name,
                        protocol = protocol,
                        downstream_url = downstream_url,
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
            downstream_url,
            protocol,
            allowed_sources,
            secret_name,
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
            .path_and_query(self.downstream_url.as_str())
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
}
