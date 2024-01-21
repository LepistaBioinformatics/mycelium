use crate::domain::{
    actors::DefaultActor,
    dtos::{
        guest::{GuestRole, Permissions},
        profile::Profile,
    },
    entities::{GuestRoleFetching, GuestRoleUpdating},
};
use clean_base::{
    entities::{FetchResponseKind, UpdatingResponseKind},
    utils::errors::{factories::use_case_err, MappedErrors},
};
use serde::{Deserialize, Serialize};
use std::{
    fmt::{Display, Formatter, Result as FmtResult},
    str::FromStr,
};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum ActionType {
    Upgrade,
    Downgrade,
}

impl Display for ActionType {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            ActionType::Upgrade => write!(f, "upgrade"),
            ActionType::Downgrade => write!(f, "downgrade"),
        }
    }
}

impl FromStr for ActionType {
    type Err = ();

    fn from_str(s: &str) -> Result<ActionType, ()> {
        match s {
            "upgrade" => Ok(ActionType::Upgrade),
            "downgrade" => Ok(ActionType::Downgrade),

            _ => Err(()),
        }
    }
}

/// This function allow users to include or remove permission from a single
/// role. Only manager users should perform such action.
pub async fn update_guest_role_permissions(
    profile: Profile,
    role_id: Uuid,
    permission: Permissions,
    action_type: ActionType,
    role_fetching_repo: Box<&dyn GuestRoleFetching>,
    role_updating_repo: Box<&dyn GuestRoleUpdating>,
) -> Result<UpdatingResponseKind<GuestRole>, MappedErrors> {
    // ? ----------------------------------------------------------------------
    // ? Check the profile permissions
    //
    // Check if the user has manager status. Return an error if not.
    // ? ----------------------------------------------------------------------

    profile.get_default_update_ids_or_error(vec![
        DefaultActor::GuestManager.to_string(),
    ])?;

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
