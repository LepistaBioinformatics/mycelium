use crate::domain::{
    dtos::{guest::GuestRole, profile::Profile},
    entities::GuestRoleFetching,
};

use clean_base::{
    entities::FetchManyResponseKind,
    utils::errors::{factories::use_case_err, MappedErrors},
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

    if !profile.is_manager {
        return use_case_err(
            "The current user has no sufficient privileges to register new 
            role."
                .to_string(),
            Some(true),
            None,
        );
    }

    // ? -----------------------------------------------------------------------
    // ? Fetch Roles
    // ? -----------------------------------------------------------------------

    guest_role_fetching_repo.list(name, role_id).await
}
