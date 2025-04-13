use crate::domain::{dtos::service::Service, entities::RoutesFetching};

use mycelium_base::{
    entities::FetchManyResponseKind, utils::errors::MappedErrors,
};
use uuid::Uuid;

#[tracing::instrument(
    name = "list_discoverable_services",
    skip(routes_fetching_repo)
)]
pub async fn list_discoverable_services(
    id: Option<Uuid>,
    name: Option<String>,
    routes_fetching_repo: Box<&dyn RoutesFetching>,
) -> Result<FetchManyResponseKind<Service>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Match upstream routes
    // ? -----------------------------------------------------------------------

    routes_fetching_repo
        .list_services(id, name, Some(true))
        .await
}
