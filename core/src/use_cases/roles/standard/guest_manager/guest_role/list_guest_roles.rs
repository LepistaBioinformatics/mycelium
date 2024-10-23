use crate::domain::{
    actors::ActorName,
    dtos::{guest_role::GuestRole, profile::Profile},
    entities::GuestRoleFetching,
};

use mycelium_base::{
    entities::FetchManyResponseKind, utils::errors::MappedErrors,
};
use uuid::Uuid;

/// List guest roles
#[tracing::instrument(name = "list_guest_roles", skip_all)]
pub async fn list_guest_roles(
    profile: Profile,
    name: Option<String>,
    role_id: Option<Uuid>,
    guest_role_fetching_repo: Box<&dyn GuestRoleFetching>,
) -> Result<FetchManyResponseKind<GuestRole>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges to create role
    // ? -----------------------------------------------------------------------

    profile.get_default_view_ids_or_error(vec![ActorName::GuestManager])?;

    // ? -----------------------------------------------------------------------
    // ? Fetch Roles
    // ? -----------------------------------------------------------------------

    guest_role_fetching_repo.list(name, role_id).await
}
