use super::{
    http::{HttpMethod, Protocol, RouteType},
    service::ClientService,
};

use actix_web::http::{uri::PathAndQuery, Uri};
use clean_base::utils::errors::{execution_err, MappedErrors};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Route {
    pub service: ClientService,
    pub id: Option<Uuid>,
    pub group: RouteType,
    pub methods: Vec<HttpMethod>,
    pub downstream_url: String,
    pub protocol: Protocol,
    pub allowed_sources: Option<Vec<String>>,
}

impl Route {
    /// Build a actix_web::http::Uri from itself.
    pub async fn build_uri(&self) -> Result<Uri, MappedErrors> {
        let host = self.service.to_owned().host;
        let path_parts = host.split("/").collect::<Vec<&str>>();
        let domain = path_parts[0];

        match Uri::builder()
            .scheme(self.protocol.to_string().as_str())
            .authority(domain)
            .path_and_query(self.downstream_url.as_str())
            .build()
        {
            Err(err) => {
                return Err(execution_err(
                    format!("Unexpected error on build URI: {}", err),
                    Some(true),
                    None,
                ))
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
                return Err(execution_err(
                    format!("Unexpected error on build URI: {}", err),
                    Some(true),
                    None,
                ))
            }
            Ok(res) => Ok(res),
        }
    }
}
