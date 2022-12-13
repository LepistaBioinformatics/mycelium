use crate::domain::{
    dtos::{
        guest::{GuestRoleDTO, PermissionsType},
        profile::ProfileDTO,
    },
    entities::{
        manager::guest_role_updating::GuestRoleUpdating,
        shared::guest_role_fetching::GuestRoleFetching,
    },
};

use agrobase::{
    entities::default_response::{FetchResponseKind, UpdatingResponseKind},
    utils::errors::{use_case_err, MappedErrors},
};
use uuid::Uuid;

pub enum ActionType {
    Upgrade,
    Downgrade,
}

/// This function allow users to include or remove permission from a single
/// role. Only manager users should perform such action.
pub async fn update_guest_role_permissions(
    profile: ProfileDTO,
    role_id: Uuid,
    permission: PermissionsType,
    action_type: ActionType,
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
                    format!("Unable to update record: {}", id.unwrap(),),
                    Some(true),
                    None,
                ));
            }
            FetchResponseKind::Found(role) => role,
        },
    };

    // ? ----------------------------------------------------------------------
    // ? Update permissions
    // ? ----------------------------------------------------------------------

    let mut updated_permissions = user_role.to_owned().permissions;

    user_role.permissions = match action_type {
        ActionType::Upgrade => {
            updated_permissions.push(permission);
            updated_permissions.dedup();
            updated_permissions
        }
        ActionType::Downgrade => {
            updated_permissions.retain(|perm| *perm != permission);
            updated_permissions
        }
    };

    // ? ----------------------------------------------------------------------
    // ? Perform the updating operation
    // ? ----------------------------------------------------------------------

    role_updating_repo.update(user_role).await
}
