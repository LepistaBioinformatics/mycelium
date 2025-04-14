use crate::domain::{dtos::route::Route, entities::RoutesRead};

use actix_web::http::uri::PathAndQuery;
use mycelium_base::{entities::FetchResponseKind, utils::errors::MappedErrors};

/// Matches the address to route
///
/// This function should be called by the main middleware router function. It
/// will try to match the address to a route and return the route if found.
#[tracing::instrument(
    name = "match_forward_address",
    skip(routes_fetching_repo)
)]
pub async fn match_forward_address(
    path: PathAndQuery,
    routes_fetching_repo: Box<&dyn RoutesRead>,
) -> Result<FetchResponseKind<Route, String>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Try to fetch routes from database
    // ? -----------------------------------------------------------------------

    routes_fetching_repo.get(path.to_owned()).await
}
