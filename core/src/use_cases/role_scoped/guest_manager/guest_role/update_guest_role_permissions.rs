use crate::domain::{
    actors::SystemActor,
    dtos::{
        guest_role::{GuestRole, Permission},
        profile::Profile,
    },
    entities::{GuestRoleFetching, GuestRoleUpdating},
};

use mycelium_base::{
    entities::{FetchResponseKind, UpdatingResponseKind},
    utils::errors::{use_case_err, MappedErrors},
};
use uuid::Uuid;

/// This function allow users to include or remove permission from a single
/// role. Only manager users should perform such action.
#[tracing::instrument(name = "update_guest_role_permission", skip_all)]
pub async fn update_guest_role_permission(
    profile: Profile,
    guest_role_id: Uuid,
    permission: Permission,
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
        .with_write_access()
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
                    id.unwrap(),
                ))
                .as_error();
            }
            FetchResponseKind::Found(role) => role,
        };

    // ? ----------------------------------------------------------------------
    // ? Update permissions
    // ? ----------------------------------------------------------------------

    user_role.permission = permission;

    // ? ----------------------------------------------------------------------
    // ? Perform the updating operation
    // ? ----------------------------------------------------------------------

    guest_role_updating_repo.update(user_role).await
}
