use crate::domain::dtos::{
    health_check::HealthCheckConfig,
    http::{HttpMethod, Protocol},
    route::Route,
    route_type::RouteType,
    service::{Service, ServiceSecret},
};

use myc_config::secret_resolver::SecretResolver;
use mycelium_base::utils::errors::{use_case_err, MappedErrors};
use serde::{Deserialize, Serialize};
use std::{mem::size_of_val, str::from_utf8};
use tokio::fs::read as t_read;
use tracing::{error, info};
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
    pub health_check: Option<HealthCheckConfig>,
    pub routes: Vec<TempRouteDTO>,
    pub secrets: Option<Vec<ServiceSecret>>,
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

/// Load configuration from YAML file
///
/// This function will load the configuration from a JSON file and return a
/// vector of routes.
///
#[tracing::instrument(name = "load_config_from_yaml")]
pub async fn load_config_from_yaml(
    source_file_path: String,
) -> Result<Vec<Route>, MappedErrors> {
    let temp_services = t_read(source_file_path)
        .await
        .map(|data| {
            match serde_yaml::from_str::<TempMainConfigDTO>(
                match from_utf8(&data) {
                    Err(err) => {
                        error!("Invalid UTF-8 sequence: {err}");
                        return use_case_err(format!(
                            "Invalid UTF-8 sequence: {err}"
                        ))
                        .as_error();
                    }
                    Ok(res) => res,
                },
            ) {
                Err(err) => {
                    error!("Invalid UTF-8 sequence: {err}");
                    return use_case_err(format!(
                        "Invalid UTF-8 sequence: {err}"
                    ))
                    .as_error();
                }
                Ok(res) => Ok(res),
            }
        })
        .unwrap();

    let db = temp_services?.services.into_iter().fold(
        Vec::<Route>::new(),
        |mut init, tmp_service| {
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

            // Check if secrets is valid
            let parsed_secrets: Option<Vec<ServiceSecret>> = match secrets {
                Some(secrets) => {
                    let mut parsed_secrets = vec![];

                    for secret in secrets {
                        let parsed_value = match secret.secret.get_or_error() {
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

            let service = Service::new(
                tmp_service.id,
                tmp_service.name.to_owned(),
                tmp_service.host.to_owned(),
                tmp_service.health_check.to_owned(),
                vec![],
                parsed_secrets.to_owned(),
            );

            init.append(
                &mut tmp_service
                    .to_owned()
                    .routes
                    .into_iter()
                    .map(|r| {
                        if let Some(secret_name) = r.secret_name.to_owned() {
                            if let Some(secrets) = parsed_secrets.to_owned() {
                                if !secrets
                                    .iter()
                                    .map(|i| i.name.to_owned())
                                    .collect::<Vec<String>>()
                                    .contains(&secret_name)
                                {
                                    panic!("Secret not found: {secret_name}");
                                }
                            }
                        }

                        Route::new(
                            r.id,
                            service.to_owned(),
                            r.group,
                            r.methods,
                            r.path,
                            r.protocol,
                            r.allowed_sources,
                            r.secret_name,
                            r.accept_insecure_routing,
                        )
                    })
                    .collect::<Vec<Route>>(),
            );

            init
        },
    );

    info!(
        "Database successfully loaded:\n
    Number of routes: {}
    In memory size: {:.6} Mb\n",
        db.len(),
        ((size_of_val(&*db) as f64 * 0.000001) as f64),
    );

    Ok(db)
}
