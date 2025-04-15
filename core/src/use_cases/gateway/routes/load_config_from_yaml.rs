use crate::domain::dtos::{
    health_check::HealthCheckConfig,
    http::{HttpMethod, Protocol},
    route::Route,
    route_type::RouteType,
    service::{Service, ServiceSecret},
};

use futures::executor::block_on;
use myc_config::secret_resolver::SecretResolver;
use mycelium_base::{dtos::Parent, utils::errors::MappedErrors};
use serde::{Deserialize, Serialize};
use std::{mem::size_of_val, str::from_utf8};
use tokio::fs::read as t_read;
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct TempMainConfigDTO {
    services: Vec<TempServiceDTO>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct TempServiceDTO {
    pub id: Option<Uuid>,
    pub name: String,
    pub host: String,
    pub discoverable: Option<bool>,
    pub description: Option<String>,
    pub openapi_path: Option<String>,
    pub health_check: Option<HealthCheckConfig>,
    pub routes: Vec<TempRouteDTO>,
    pub secrets: Option<Vec<ServiceSecret>>,
}

impl TempServiceDTO {
    fn to_service(self) -> Service {
        Service::new(
            self.id.clone(),
            self.name.clone(),
            self.host.clone(),
            self.discoverable.clone(),
            self.description.clone(),
            self.openapi_path.clone(),
            self.health_check.clone(),
            vec![],
            self.secrets,
        )
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct TempRouteDTO {
    pub id: Option<Uuid>,
    pub group: RouteType,
    pub methods: Vec<HttpMethod>,
    pub path: String,
    pub protocol: Protocol,
    pub allowed_sources: Option<Vec<String>>,
    pub secret_name: Option<String>,
    pub accept_insecure_routing: Option<bool>,
}

impl TempRouteDTO {
    fn to_route(self, service: Service) -> Route {
        Route::new(
            self.id,
            service,
            self.group,
            self.methods,
            self.path,
            self.protocol,
            self.allowed_sources,
            self.secret_name,
            self.accept_insecure_routing,
        )
    }
}

/// Load configuration from YAML file
///
/// This function will load the configuration from a JSON file and return a
/// vector of routes.
///
#[tracing::instrument(name = "load_config_from_yaml")]
pub async fn load_config_from_yaml(
    source_file_path: String,
) -> Result<Vec<Service>, MappedErrors> {
    let services_binding = t_read(source_file_path).await.map(|data| {
        let decoded_string = from_utf8(&data)
            .map_err(|err| panic!("Invalid UTF-8 sequence: {err}"))
            .unwrap();

        serde_yaml::from_str::<TempMainConfigDTO>(&decoded_string)
            .map_err(|err| panic!("Invalid YAML: {err}"))
            .unwrap()
    });

    let tmp_services = services_binding.unwrap().services;

    //
    // Check if secrets are valid
    //
    let services = tmp_services
        .into_iter()
        .map(|tmp_service| {
            let secrets = if let Some(secrets) = tmp_service.to_owned().secrets
            {
                let secrets =
                    secrets.into_iter().collect::<Vec<ServiceSecret>>();

                match secrets.is_empty() {
                    true => None,
                    false => Some(secrets),
                }
            } else {
                None
            };

            match secrets {
                Some(secrets) => {
                    let mut parsed_secrets = vec![];

                    for secret in secrets {
                        let secret_value =
                            block_on(secret.secret.async_get_or_error());

                        let parsed_value = match secret_value {
                            Ok(res) => res,
                            Err(err) => {
                                panic!("Error on check secrets: {err}");
                            }
                        };

                        parsed_secrets.push(ServiceSecret::new(
                            secret.name,
                            SecretResolver::Value(parsed_value),
                        ));
                    }

                    Some(parsed_secrets)
                }
                None => None,
            };

            let routes = tmp_service
                .routes
                .clone()
                .into_iter()
                .map(|route| {
                    let mut tmp_route = route
                        .to_owned()
                        .to_route(tmp_service.to_owned().to_service());

                    let mut local_service = tmp_service.to_owned().to_service();
                    local_service.routes = vec![];

                    tmp_route.service = Parent::Record(local_service);

                    tmp_route
                })
                .collect::<Vec<Route>>();

            let mut tmp_service = tmp_service.to_owned().to_service();

            tmp_service.routes = routes;

            tmp_service
        })
        .collect::<Vec<Service>>();

    tracing::info!(
        "Database successfully loaded:\n
    Number of services: {}
    In memory size: {:.6} Mb\n",
        services.len(),
        ((size_of_val(&*services) as f64 * 0.000001) as f64),
    );

    Ok(services)
}
