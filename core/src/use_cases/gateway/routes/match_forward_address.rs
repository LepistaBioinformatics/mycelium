use crate::domain::{dtos::route::Route, entities::RoutesFetching};

use actix_web::http::uri::PathAndQuery;
use log::debug;
use mycelium_base::{
    entities::FetchManyResponseKind, utils::errors::MappedErrors,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RoutesMatchResponseEnum {
    PathNotFound(String),
    MultipleAssociatedPaths(String),
    Found(Route),
}

/// Matches the address to route
///
/// This function should be called by the main middleware router function. It
/// will try to match the address to a route and return the route if found.
pub async fn match_forward_address(
    path: PathAndQuery,
    routes_fetching_repo: Box<&dyn RoutesFetching>,
) -> Result<RoutesMatchResponseEnum, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Try to fetch routes from database
    // ? -----------------------------------------------------------------------

    let routes = match routes_fetching_repo.list(path.to_owned()).await? {
        FetchManyResponseKind::NotFound => {
            return Ok(RoutesMatchResponseEnum::PathNotFound(format!(
                "There is no registered paths for: {path}",
            )))
        }
        FetchManyResponseKind::Found(res) => res,
        _ => panic!(
            "Paginated routes parsing not implemented in 
            `match_forward_address` use-case."
        ),
    };

    debug!("routes: {:?}", routes);

    if routes.len() == 0 {
        return Ok(RoutesMatchResponseEnum::PathNotFound(format!(
            "There is no registered paths for: {path}"
        )));
    }

    if routes.len() > 1 {
        return Ok(RoutesMatchResponseEnum::MultipleAssociatedPaths(format!(
            "Multiple paths registered for the specified path: {path}",
        )));
    }

    // ? -----------------------------------------------------------------------
    // ? Try to fetch routes from database
    // ? -----------------------------------------------------------------------

    Ok(RoutesMatchResponseEnum::Found(routes[0].to_owned()))
}
