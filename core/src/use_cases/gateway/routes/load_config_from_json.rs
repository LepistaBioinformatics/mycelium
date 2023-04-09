use crate::domain::dtos::{
    health_check::HealthCheckConfig,
    http::{HttpMethod, Protocol, RouteType},
    route::Route,
    service::ClientService,
};

use clean_base::utils::errors::{factories::use_case_err, MappedErrors};
use log::{debug, error, info};
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
    pub health_check: Option<HealthCheckConfig>,
    pub routes: Vec<TempRouteDTO>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct TempRouteDTO {
    pub id: Option<Uuid>,
    pub group: RouteType,
    pub methods: Vec<HttpMethod>,
    pub downstream_url: String,
    pub protocol: Protocol,

    // Self optional fields
    pub allowed_sources: Option<Vec<String>>,
}

/// This function load the in-memory database from JSON file.
pub async fn load_config_from_json(
    source_file_path: String,
) -> Result<Vec<Route>, MappedErrors> {
    let temp_services = t_read(source_file_path)
        .await
        .map(|data| {
            match serde_yaml::from_str::<TempMainConfigDTO>(
                match from_utf8(&data) {
                    Err(err) => {
                        error!("Invalid UTF-8 sequence: {err}");
                        return Err(use_case_err(
                            format!("Invalid UTF-8 sequence: {err}"),
                            None,
                            None,
                        ));
                    }
                    Ok(res) => res,
                },
            ) {
                Err(err) => {
                    error!("Invalid UTF-8 sequence: {err}");
                    return Err(use_case_err(
                        format!("Invalid UTF-8 sequence: {err}"),
                        None,
                        None,
                    ));
                }
                Ok(res) => Ok(res),
            }
        })
        .unwrap();

    debug!("temp_services: {:?}", temp_services);

    let tem_service_vec = match temp_services {
        Err(err) => return err,
        Ok(res) => res,
    };

    debug!("tem_service_vec: {:?}", tem_service_vec);

    let db = tem_service_vec.services.into_iter().fold(
        Vec::<Route>::new(),
        |mut init, tmp_service| {
            let service = ClientService {
                id: match tmp_service.id {
                    None => Some(Uuid::new_v4()),
                    Some(id) => Some(id),
                },
                name: tmp_service.name.to_owned(),
                host: tmp_service.host.to_owned(),
                health_check: tmp_service.health_check.to_owned(),
                routes: vec![],
            };

            init.append(
                &mut tmp_service
                    .to_owned()
                    .routes
                    .into_iter()
                    .map(|r| Route {
                        service: service.to_owned(),
                        id: match r.id {
                            None => Some(Uuid::new_v4()),
                            Some(id) => Some(id),
                        },
                        group: r.group,
                        methods: r.methods,
                        downstream_url: r.downstream_url,
                        protocol: r.protocol,
                        allowed_sources: r.allowed_sources,
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
