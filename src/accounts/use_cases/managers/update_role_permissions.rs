use crate::domain::{
    dtos::{
        guest::{PermissionsType, UserRoleDTO},
        profile::ProfileDTO,
    },
    entities::{
        manager::{
            user_role_fetching::UserRoleFetching,
            user_role_updating::UserRoleUpdating,
        },
        shared::default_responses::{FetchResponse, UpdateResponse},
    },
    utils::errors::{use_case_err, MappedErrors},
};

use uuid::Uuid;

pub enum ActionType {
    Upgrade,
    Downgrade,
}

/// This function allow users to include or remove permission from a single
/// role. Only manager users should perform such action.
pub async fn update_role_permissions(
    profile: ProfileDTO,
    role_id: Uuid,
    permission: PermissionsType,
    action_type: ActionType,
    role_fetching_repo: Box<&dyn UserRoleFetching>,
    role_updating_repo: Box<&dyn UserRoleUpdating>,
) -> Result<UpdateResponse<UserRoleDTO>, MappedErrors> {
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
            FetchResponse::NotFound(id, msg) => {
                return Err(use_case_err(
                    format!(
                        "Unable to update record ({}): {}",
                        id,
                        msg.unwrap()
                    ),
                    Some(true),
                    None,
                ));
            }
            FetchResponse::Found(role) => role,
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
