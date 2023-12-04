use crate::domain::{
    actors::DefaultActor,
    dtos::{profile::Profile, role::Role},
    entities::RoleFetching,
};

use clean_base::{
    entities::FetchManyResponseKind, utils::errors::MappedErrors,
};

/// List available roles
pub async fn list_roles(
    profile: Profile,
    name: Option<String>,
    roles_fetching_repo: Box<&dyn RoleFetching>,
) -> Result<FetchManyResponseKind<Role>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges to create role
    // ? -----------------------------------------------------------------------

    profile
        .get_view_ids_or_error(vec![DefaultActor::GuestManager.to_string()])?;

    // ? -----------------------------------------------------------------------
    // ? Fetch Roles
    // ? -----------------------------------------------------------------------

    roles_fetching_repo.list(name).await
}
