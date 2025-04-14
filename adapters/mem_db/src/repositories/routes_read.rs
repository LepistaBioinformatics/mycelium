use super::shared::extract_path_parts;
use crate::models::config::DbPoolProvider;

use actix_web::http::uri::PathAndQuery;
use async_trait::async_trait;
use myc_core::domain::{dtos::route::Route, entities::RoutesRead};
use mycelium_base::{
    dtos::UntaggedChildren,
    entities::{FetchManyResponseKind, FetchResponseKind},
    utils::errors::{fetching_err, MappedErrors},
};
use shaku::Component;
use std::sync::Arc;
use uuid::Uuid;
use wildmatch::WildMatch;

#[derive(Component)]
#[shaku(interface = RoutesRead)]
pub struct RoutesFetchingMemDbRepo {
    #[shaku(inject)]
    pub db_config: Arc<dyn DbPoolProvider>,
}

#[async_trait]
impl RoutesRead for RoutesFetchingMemDbRepo {
    async fn get(
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
            .flat_map(|service| service.routes)
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
            return fetching_err(
                "Multiple routes found for the specified path.",
            )
            .with_exp_true()
            .as_error();
        }

        Ok(FetchResponseKind::Found(
            response.first().unwrap().to_owned(),
        ))
    }

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
            .flat_map(|service| service.routes)
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
