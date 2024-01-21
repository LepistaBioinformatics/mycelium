use crate::domain::{
    actors::DefaultActor,
    dtos::{guest::GuestRole, profile::Profile},
    entities::GuestRoleFetching,
};

use clean_base::{
    entities::FetchManyResponseKind, utils::errors::MappedErrors,
};
use uuid::Uuid;

/// List guest roles
pub async fn list_guest_roles(
    profile: Profile,
    name: Option<String>,
    role_id: Option<Uuid>,
    guest_role_fetching_repo: Box<&dyn GuestRoleFetching>,
) -> Result<FetchManyResponseKind<GuestRole>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges to create role
    // ? -----------------------------------------------------------------------

    profile.get_default_view_ids_or_error(vec![
        DefaultActor::GuestManager.to_string()
    ])?;

    // ? -----------------------------------------------------------------------
    // ? Fetch Roles
    // ? -----------------------------------------------------------------------

    guest_role_fetching_repo.list(name, role_id).await
}
