use crate::domain::{
    actors::SystemActor,
    dtos::{guest_role::GuestRole, profile::Profile},
    entities::{GuestRoleFetching, GuestRoleUpdating},
};

use mycelium_base::{
    entities::{FetchResponseKind, UpdatingResponseKind},
    utils::errors::{use_case_err, MappedErrors},
};
use uuid::Uuid;

/// This function allows only the update of name and description attributes of
/// a single role.
#[tracing::instrument(
    name = "update_guest_role_name_and_description",
    skip_all
)]
pub async fn update_guest_role_name_and_description(
    profile: Profile,
    name: Option<String>,
    description: Option<String>,
    guest_role_id: Uuid,
    guest_role_fetching_repo: Box<&dyn GuestRoleFetching>,
    guest_role_updating_repo: Box<&dyn GuestRoleUpdating>,
) -> Result<UpdatingResponseKind<GuestRole>, MappedErrors> {
    // ? ----------------------------------------------------------------------
    // ? Check the profile permissions
    //
    // Check if the user has manager status. Return an error if not.
    // ? ----------------------------------------------------------------------

    profile
        .with_system_accounts_access()
        .with_read_write_access()
        .with_roles(vec![SystemActor::GuestsManager])
        .get_ids_or_error()?;

    // ? ----------------------------------------------------------------------
    // ? Fetch role from data persistence layer
    // ? ----------------------------------------------------------------------

    let mut user_role =
        match guest_role_fetching_repo.get(guest_role_id).await? {
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

    guest_role_updating_repo.update(user_role).await
}
