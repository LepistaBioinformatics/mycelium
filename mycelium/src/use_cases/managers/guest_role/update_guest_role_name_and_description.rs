use crate::domain::{
    dtos::{guest::GuestRoleDTO, profile::ProfileDTO},
    entities::{
        manager::guest_role_updating::GuestRoleUpdating,
        shared::guest_role_fetching::GuestRoleFetching,
    },
};

use clean_base::{
    entities::default_response::{FetchResponseKind, UpdatingResponseKind},
    utils::errors::{use_case_err, MappedErrors},
};
use uuid::Uuid;

/// This function allows only the update of name and description attributes of
/// a single role.
pub async fn update_guest_role_name_and_description(
    profile: ProfileDTO,
    name: Option<String>,
    description: Option<String>,
    role_id: Uuid,
    role_fetching_repo: Box<&dyn GuestRoleFetching>,
    role_updating_repo: Box<&dyn GuestRoleUpdating>,
) -> Result<UpdatingResponseKind<GuestRoleDTO>, MappedErrors> {
    // ? ----------------------------------------------------------------------
    // ? Check the profile permissions
    //
    // Check if the user has manager status. Return an error if not.
    // ? ----------------------------------------------------------------------

    if !profile.is_manager {
        return Err(use_case_err(
            "Only manager user could perform such operation.".to_string(),
            Some(true),
            None,
        ));
    };

    // ? ----------------------------------------------------------------------
    // ? Fetch role from data persistence layer
    // ? ----------------------------------------------------------------------

    let mut user_role = match role_fetching_repo.get(role_id).await {
        Err(err) => return Err(err),
        Ok(res) => match res {
            FetchResponseKind::NotFound(id) => {
                return Err(use_case_err(
                    format!("Unable to update record: {}", id.unwrap()),
                    Some(true),
                    None,
                ));
            }
            FetchResponseKind::Found(role) => role,
        },
    };

    // ? ----------------------------------------------------------------------
    // ? Update value of fetched object
    // ? ----------------------------------------------------------------------

    if name.is_some() {
        user_role.name = name.unwrap();
    };

    if description.is_some() {
        user_role.description = description.unwrap();
    };

    // ? ----------------------------------------------------------------------
    // ? Perform the updating operation
    // ? ----------------------------------------------------------------------

    role_updating_repo.update(user_role).await
}
