use crate::domain::{
    actors::SystemActor::*,
    dtos::{
        guest_role::Permission, service::Service,
        token::RoleScopedConnectionString,
    },
    entities::RoutesFetching,
};

use mycelium_base::{
    entities::FetchManyResponseKind, utils::errors::MappedErrors,
};
use uuid::Uuid;

#[tracing::instrument(
    name = "list_discoverable_services",
    skip(routes_fetching_repo)
)]
pub async fn list_discoverable_services(
    scope: RoleScopedConnectionString,
    id: Option<Uuid>,
    name: Option<String>,
    routes_fetching_repo: Box<&dyn RoutesFetching>,
) -> Result<FetchManyResponseKind<Service>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges to create role
    // ? -----------------------------------------------------------------------

    scope.contain_enough_permissioned_roles(vec![(
        Service.to_string(),
        Permission::Read,
    )])?;

    // ? -----------------------------------------------------------------------
    // ? Match upstream routes
    // ? -----------------------------------------------------------------------

    routes_fetching_repo
        .list_services(id, name, Some(true))
        .await
}
