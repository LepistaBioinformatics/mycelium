use super::shared::extract_path_parts;
use crate::models::config::DbPoolProvider;

use async_trait::async_trait;
use http::uri::PathAndQuery;
use myc_core::domain::{dtos::route::Route, entities::RoutesRead};
use mycelium_base::{
    dtos::Parent,
    entities::{FetchManyResponseKind, FetchResponseKind},
    utils::errors::{fetching_err, MappedErrors},
};
use shaku::Component;
use std::{sync::Arc, vec};
use uuid::Uuid;
use wildmatch::WildMatch;

#[derive(Component)]
#[shaku(interface = RoutesRead)]
pub struct RoutesReadMemDbRepo {
    #[shaku(inject)]
    pub db_config: Arc<dyn DbPoolProvider>,
}

#[async_trait]
impl RoutesRead for RoutesReadMemDbRepo {
    #[tracing::instrument(name = "match_single_path_or_error", skip_all)]
    async fn match_single_path_or_error(
        &self,
        path: PathAndQuery,
    ) -> Result<FetchResponseKind<Route, String>, MappedErrors> {
        let db = self.db_config.get_services_db();

        if db.len() == 0 {
            return fetching_err("Routes already not initialized.".to_string())
                .as_error();
        }

        let (service_name, rest) = extract_path_parts(path);

        let response = db
            .into_iter()
            .filter(|service| {
                //
                // Check the match between the registered service name and the
                // requested service name.
                //
                service.name == service_name
            })
            .map(|service| {
                let mut tmp_service = service.clone();
                let routes = service.routes.clone();
                tmp_service.routes = vec![];

                (tmp_service, routes)
            })
            .flat_map(|(service, routes)| {
                routes
                    .into_iter()
                    .map(|route| {
                        let mut tmp_route = route.clone();
                        tmp_route.service = Parent::Record(service.clone());
                        tmp_route
                    })
                    .collect::<Vec<Route>>()
            })
            .filter(|route| {
                //
                // Check the match between the registered route and the
                // requested route.
                //
                WildMatch::new(&route.path.as_str()).matches(rest.as_str())
            })
            .collect::<Vec<Route>>();

        if response.len() == 0 {
            return Ok(FetchResponseKind::NotFound(None));
        }

        if response.len() > 1 {
            return fetching_err(format!(
                "Multiple routes found for the specified path: {}",
                response
                    .iter()
                    .map(|r| r.path.clone())
                    .collect::<Vec<String>>()
                    .join(", ")
            ))
            .with_exp_true()
            .as_error();
        }

        Ok(FetchResponseKind::Found(
            response.first().unwrap().to_owned(),
        ))
    }

    #[tracing::instrument(name = "list_routes", skip_all)]
    async fn list_routes_paginated(
        &self,
        id: Option<Uuid>,
        name: Option<String>,
        page_size: Option<i32>,
        skip: Option<i32>,
    ) -> Result<FetchManyResponseKind<Route>, MappedErrors> {
        let db = self.db_config.get_services_db();

        if db.len() == 0 {
            return fetching_err("Routes already not initialized.".to_string())
                .as_error();
        }

        let page_size = page_size.unwrap_or(10) as i64;
        let skip = skip.unwrap_or(0) as i64;

        let routes_db = db
            .into_iter()
            .map(|service| {
                let mut tmp_service = service.clone();
                let routes = service.routes.clone();
                tmp_service.routes = vec![];

                (tmp_service, routes)
            })
            .flat_map(|(service, routes)| {
                routes
                    .into_iter()
                    .map(|route| {
                        let mut tmp_route = route.clone();
                        tmp_route.service = Parent::Record(service.clone());
                        tmp_route
                    })
                    .collect::<Vec<Route>>()
            })
            .collect::<Vec<Route>>();

        let mut binding_routes_db = routes_db.clone();
        binding_routes_db.sort_by_key(|route| route.path.clone());

        let response = binding_routes_db
            .into_iter()
            .filter(|route| {
                //
                // Check the match between the registered route id and the
                // requested route id.
                //
                if let Some(id) = &id {
                    route.id == Some(*id)
                } else {
                    true
                }
            })
            .filter(|route| {
                //
                // Check the match between the registered route and the
                // requested route.
                //
                if let Some(name) = &name {
                    WildMatch::new(&route.path.as_str()).matches(name.as_str())
                } else {
                    true
                }
            })
            .skip(skip as usize)
            .take(page_size as usize)
            .into_iter()
            .collect::<Vec<Route>>();

        if response.len() == 0 {
            return Ok(FetchManyResponseKind::NotFound);
        }

        Ok(FetchManyResponseKind::FoundPaginated {
            count: response.len() as i64,
            skip: Some(skip),
            size: Some(page_size),
            records: response,
        })
    }

    #[tracing::instrument(name = "list_routes", skip_all)]
    async fn list_routes(
        &self,
        id: Option<Uuid>,
        name: Option<String>,
    ) -> Result<FetchManyResponseKind<Route>, MappedErrors> {
        let db = self.db_config.get_services_db();

        if db.len() == 0 {
            return fetching_err("Routes already not initialized.".to_string())
                .as_error();
        }

        let response = db
            .into_iter()
            .map(|service| {
                let mut tmp_service = service.clone();
                let routes = service.routes.clone();
                tmp_service.routes = vec![];

                (tmp_service, routes)
            })
            .flat_map(|(service, routes)| {
                routes
                    .into_iter()
                    .map(|route| {
                        let mut tmp_route = route.clone();
                        tmp_route.service = Parent::Record(service.clone());
                        tmp_route
                    })
                    .collect::<Vec<Route>>()
            })
            .filter(|route| {
                //
                // Check the match between the registered route id and the
                // requested route id.
                //
                if let Some(id) = &id {
                    route.id == Some(*id)
                } else {
                    true
                }
            })
            .filter(|route| {
                //
                // Check the match between the registered route and the
                // requested route.
                //
                if let Some(name) = &name {
                    WildMatch::new(&route.path.as_str()).matches(name.as_str())
                } else {
                    true
                }
            })
            .into_iter()
            .collect::<Vec<Route>>();

        if response.len() == 0 {
            return Ok(FetchManyResponseKind::NotFound);
        }

        Ok(FetchManyResponseKind::Found(response))
    }
}
