use crate::domain::{
    actors::ActorName,
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
    role_id: Uuid,
    permission: Permission,
    role_fetching_repo: Box<&dyn GuestRoleFetching>,
    role_updating_repo: Box<&dyn GuestRoleUpdating>,
) -> Result<UpdatingResponseKind<GuestRole>, MappedErrors> {
    // ? ----------------------------------------------------------------------
    // ? Check the profile permissions
    //
    // Check if the user has manager status. Return an error if not.
    // ? ----------------------------------------------------------------------

    profile
        .get_default_read_write_ids_or_error(vec![ActorName::GuestManager])?;

    // ? ----------------------------------------------------------------------
    // ? Fetch role from data persistence layer
    // ? ----------------------------------------------------------------------

    let mut user_role = match role_fetching_repo.get(role_id).await? {
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

    role_updating_repo.update(user_role).await
}
