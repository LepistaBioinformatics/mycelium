use crate::domain::{
    dtos::{profile::Profile, role::Role},
    entities::RoleFetching,
};

use clean_base::{
    entities::FetchManyResponseKind,
    utils::errors::{factories::use_case_err, MappedErrors},
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

    if !profile.is_manager {
        return use_case_err(
            "The current user has no sufficient privileges to register new 
            role."
                .to_string(),
        )
        .as_error();
    }

    // ? -----------------------------------------------------------------------
    // ? Fetch Roles
    // ? -----------------------------------------------------------------------

    roles_fetching_repo.list(name).await
}
