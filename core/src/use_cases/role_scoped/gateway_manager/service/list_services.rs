use crate::domain::{
    actors::SystemActor,
    dtos::{profile::Profile, service::Service},
    entities::ServiceRead,
};

use mycelium_base::{
    entities::FetchManyResponseKind, utils::errors::MappedErrors,
};
use uuid::Uuid;

#[tracing::instrument(
    name = "list_services",
    fields(profile_id = %profile.acc_id),
    skip(profile, service_read_repo)
)]
pub async fn list_services(
    profile: Profile,
    id: Option<Uuid>,
    name: Option<String>,
    service_read_repo: Box<&dyn ServiceRead>,
) -> Result<FetchManyResponseKind<Service>, MappedErrors> {
    // ? ----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges to create role
    // ? ----------------------------------------------------------------------

    profile
        .with_system_accounts_access()
        .with_read_access()
        .with_roles(vec![SystemActor::GatewayManager])
        .get_ids_or_error()?;

    // ? ----------------------------------------------------------------------
    // ? Match upstream routes
    // ? ----------------------------------------------------------------------

    service_read_repo.list_services(id, name, None).await
}
