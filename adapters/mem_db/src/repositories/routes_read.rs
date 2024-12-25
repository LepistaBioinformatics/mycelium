use actix_web::http::uri::PathAndQuery;
use async_trait::async_trait;
use myc_core::{
    domain::{dtos::route::Route, entities::RoutesFetching},
    settings::ROUTES,
};
use mycelium_base::{
    dtos::Parent,
    entities::{FetchManyResponseKind, FetchResponseKind},
    utils::errors::{fetching_err, MappedErrors},
};
use shaku::Component;
use tracing::{error, warn};
use uuid::Uuid;
use wildmatch::WildMatch;

#[derive(Component)]
#[shaku(interface = RoutesFetching)]
pub struct RoutesFetchingMemDbRepo {}

#[async_trait]
impl RoutesFetching for RoutesFetchingMemDbRepo {
    async fn get(
        &self,
        path: PathAndQuery,
    ) -> Result<FetchResponseKind<Route, String>, MappedErrors> {
        let db = ROUTES.lock().await.clone();

        if db.len() == 0 {
            return fetching_err("Routes already not initialized.".to_string())
                .as_error();
        }

        let path_string = path.to_string();
        let path_parts = path_string.split("/").collect::<Vec<&str>>();
        let service_name = path_parts[1];
        let rest = path_string.replace(&format!("/{}", service_name), "");

        let response = db
            .into_iter()
            .filter(|route| {
                let service = match &route.service {
                    Parent::Record(service) => service,
                    Parent::Id(_) => {
                        error!(
                            "Service not found when trying to match the route with the route: {:?}", 
                            route.id.to_owned()
                        );

                        return false;
                    }
                };

                // Check the match between the registered route and the
                // requested route.
                let path_match = WildMatch::new(&route.path.as_str())
                    .matches(rest.as_str());

                // Check the match between the registered service name and the
                // requested service name.
                let service_name_match =
                    service.to_owned().name == service_name;

                // Check the match between the service and the route.
                path_match && service_name_match
            })
            .collect::<Vec<Route>>();

        if response.len() == 0 {
            return Ok(FetchResponseKind::NotFound(None));
        }

        if response.len() > 1 {
            return fetching_err(
                "Multiple routes found for the specified path.".to_string(),
            )
            .as_error();
        }

        Ok(FetchResponseKind::Found(
            response.first().unwrap().to_owned(),
        ))
    }

    async fn list_by_service(
        &self,
        id: Option<Uuid>,
        name: Option<String>,
        include_service_details: Option<bool>,
    ) -> Result<FetchManyResponseKind<Route>, MappedErrors> {
        let db = ROUTES.lock().await.clone();

        if db.len() == 0 {
            return fetching_err("Routes already not initialized.".to_string())
                .as_error();
        }

        let response = db
            .into_iter()
            .filter(|route| {
                let service = match &route.service {
                    Parent::Record(service) => service,
                    Parent::Id(_) => {
                        error!(
                            "Service not found when trying to match the route with the route: {:?}", 
                            route.id.to_owned()
                        );

                        return false;
                    }
                };

                // Check the match between the registered route and the
                // requested route.
                let id_match = match id {
                    Some(id) => {
                        let service_id =
                            if let Some(id) = service.to_owned().id {
                                id
                            } else {
                                warn!(
                                    "Service id not found when trying to match the route with the service: {}", 
                                    service.to_owned().name
                                );

                                return false;
                            };

                        service_id == id
                    }
                    None => true,
                };

                // Check the match between the registered service name and the
                // requested service name.
                let name_match = match &name {
                    Some(name) => service.to_owned().name == *name,
                    None => true,
                };

                // Check the match between the service and the route.
                id_match && name_match
            })
            .collect::<Vec<Route>>()
            .into_iter()
            .map(|route| {
                let service_id = match &route.service {
                    Parent::Record(service) => match service.id {
                        Some(id) => id,
                        None => {
                            error!(
                                "Service id not found when trying to match the route with the service: {}", 
                                service.name
                            );

                            return route;
                        }
                    },
                    Parent::Id(id) => id.to_owned()
                };

                if let Some(true) = include_service_details {
                    route
                } else {
                    let mut route = route;
                    route.service = Parent::Id(service_id);
                    route
                }
            })
            .collect::<Vec<Route>>();

        if response.len() == 0 {
            return Ok(FetchManyResponseKind::NotFound);
        }

        Ok(FetchManyResponseKind::Found(response))
    }
}
