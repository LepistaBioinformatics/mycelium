use actix_web::http::uri::PathAndQuery;
use async_trait::async_trait;
use clean_base::{
    entities::default_response::FetchManyResponseKind,
    utils::errors::{fetching_err, MappedErrors},
};
use myc_core::{
    domain::{dtos::route::Route, entities::RoutesFetching},
    settings::ROUTES,
};
use shaku::Component;
use wildmatch::WildMatch;

#[derive(Component)]
#[shaku(interface = RoutesFetching)]
pub struct RoutesFetchingMemDbRepo {}

#[async_trait]
impl RoutesFetching for RoutesFetchingMemDbRepo {
    async fn list(
        &self,
        path: PathAndQuery,
    ) -> Result<FetchManyResponseKind<Route>, MappedErrors> {
        let db = ROUTES.lock().await.clone();

        if db.len() == 0 {
            return Err(fetching_err(
                "Routes already not initialized.".to_string(),
                Some(true),
                None,
            ));
        }

        let path_string = path.to_string();
        let path_parts = path_string.split("/").collect::<Vec<&str>>();
        let service_name = path_parts[1];
        let rest = path_string.replace(&format!("/{}", service_name), "");

        let response = db
            .into_iter()
            .filter(|route| {
                // Check the match between the registered route and the
                // requested route.
                let path_match = WildMatch::new(&route.downstream_url.as_str())
                    .matches(rest.as_str());

                // Check the match between the registered service name and the
                // requested service name.
                let service_name_match =
                    route.service.to_owned().name == service_name;

                // Check the match between the service and the route.
                path_match && service_name_match
            })
            .collect::<Vec<Route>>();

        if response.len() == 0 {
            return Ok(FetchManyResponseKind::NotFound);
        }

        Ok(FetchManyResponseKind::Found(response))
    }
}
