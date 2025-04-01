use crate::domain::{
    actors::SystemActor,
    dtos::{guest_role::GuestRole, profile::Profile},
    entities::GuestRoleFetching,
};

use mycelium_base::{
    entities::FetchManyResponseKind, utils::errors::MappedErrors,
};

/// List guest roles
#[tracing::instrument(name = "list_guest_roles", skip_all)]
pub async fn list_guest_roles(
    profile: Profile,
    name: Option<String>,
    slug: Option<String>,
    system: Option<bool>,
    page_size: Option<i32>,
    skip: Option<i32>,
    guest_role_fetching_repo: Box<&dyn GuestRoleFetching>,
) -> Result<FetchManyResponseKind<GuestRole>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges to create role
    // ? -----------------------------------------------------------------------

    profile
        .with_system_accounts_access()
        .with_read_access()
        .with_roles(vec![SystemActor::GuestsManager])
        .get_ids_or_error()?;

    // ? -----------------------------------------------------------------------
    // ? Fetch Roles
    // ? -----------------------------------------------------------------------

    guest_role_fetching_repo
        .list(name, slug, system, page_size, skip)
        .await
}
