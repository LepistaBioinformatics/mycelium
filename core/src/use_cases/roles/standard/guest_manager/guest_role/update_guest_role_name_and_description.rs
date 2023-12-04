use crate::domain::{
    actors::DefaultActor,
    dtos::{guest::GuestRole, profile::Profile},
    entities::{GuestRoleFetching, GuestRoleUpdating},
};

use clean_base::{
    entities::{FetchResponseKind, UpdatingResponseKind},
    utils::errors::{factories::use_case_err, MappedErrors},
};
use uuid::Uuid;

/// This function allows only the update of name and description attributes of
/// a single role.
pub async fn update_guest_role_name_and_description(
    profile: Profile,
    name: Option<String>,
    description: Option<String>,
    role_id: Uuid,
    role_fetching_repo: Box<&dyn GuestRoleFetching>,
    role_updating_repo: Box<&dyn GuestRoleUpdating>,
) -> Result<UpdatingResponseKind<GuestRole>, MappedErrors> {
    // ? ----------------------------------------------------------------------
    // ? Check the profile permissions
    //
    // Check if the user has manager status. Return an error if not.
    // ? ----------------------------------------------------------------------

    profile.get_update_ids_or_error(vec![
        DefaultActor::GuestManager.to_string()
    ])?;

    // ? ----------------------------------------------------------------------
    // ? Fetch role from data persistence layer
    // ? ----------------------------------------------------------------------

    let mut user_role = match role_fetching_repo.get(role_id).await? {
        FetchResponseKind::NotFound(id) => {
            return use_case_err(format!(
                "Unable to update record: {}",
                id.unwrap()
            ))
            .as_error();
        }
        FetchResponseKind::Found(role) => role,
    };

    // ? ----------------------------------------------------------------------
    // ? Update value of fetched object
    // ? ----------------------------------------------------------------------

    if name.is_some() {
        user_role.name = name.unwrap();
    };

    if description.is_some() {
        user_role.description = description;
    };

    // ? ----------------------------------------------------------------------
    // ? Perform the updating operation
    // ? ----------------------------------------------------------------------

    role_updating_repo.update(user_role).await
}
