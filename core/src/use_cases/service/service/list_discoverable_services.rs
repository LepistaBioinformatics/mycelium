use crate::domain::{dtos::service::Service, entities::ServiceRead};

use mycelium_base::{
    entities::FetchManyResponseKind, utils::errors::MappedErrors,
};
use uuid::Uuid;

#[tracing::instrument(
    name = "list_discoverable_services",
    skip(service_read_repo)
)]
pub async fn list_discoverable_services(
    id: Option<Uuid>,
    name: Option<String>,
    service_read_repo: Box<&dyn ServiceRead>,
) -> Result<FetchManyResponseKind<Service>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Match upstream routes
    // ? -----------------------------------------------------------------------

    service_read_repo.list_services(id, name, Some(true)).await
}
