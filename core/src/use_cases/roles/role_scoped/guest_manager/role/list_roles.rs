use crate::domain::{
    actors::ActorName,
    dtos::{profile::Profile, role::Role},
    entities::RoleFetching,
};

use mycelium_base::{
    entities::FetchManyResponseKind, utils::errors::MappedErrors,
};

/// List available roles
#[tracing::instrument(name = "list_roles", skip(profile, roles_fetching_repo))]
pub async fn list_roles(
    profile: Profile,
    name: Option<String>,
    roles_fetching_repo: Box<&dyn RoleFetching>,
) -> Result<FetchManyResponseKind<Role>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges to create role
    // ? -----------------------------------------------------------------------

    profile.get_default_read_ids_or_error(vec![
        ActorName::GuestManager.to_string()
    ])?;

    // ? -----------------------------------------------------------------------
    // ? Fetch Roles
    // ? -----------------------------------------------------------------------

    roles_fetching_repo.list(name).await
}
