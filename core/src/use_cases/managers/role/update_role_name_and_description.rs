use crate::domain::{
    dtos::{profile::Profile, role::Role},
    entities::{RoleFetching, RoleUpdating},
};
use clean_base::{
    entities::default_response::{FetchResponseKind, UpdatingResponseKind},
    utils::errors::{use_case_err, MappedErrors},
};
use uuid::Uuid;

/// Update the role name and description.
///
/// This function would be allowed only by manager users.
pub async fn update_role_name_and_description(
    profile: Profile,
    role_id: Uuid,
    name: String,
    description: String,
    role_fetching_repo: Box<&dyn RoleFetching>,
    role_updating_repo: Box<&dyn RoleUpdating>,
) -> Result<UpdatingResponseKind<Role>, MappedErrors> {
    // ? ----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges to create role
    // ? ----------------------------------------------------------------------

    if !profile.is_manager {
        return Err(use_case_err(
            "The current user has no sufficient privileges to update roles."
                .to_string(),
            Some(true),
            None,
        ));
    }

    // ? ----------------------------------------------------------------------
    // ? Fetch desired role
    // ? ----------------------------------------------------------------------

    let mut role = match role_fetching_repo.get(role_id).await {
        Err(err) => return Err(err),
        Ok(res) => match res {
            FetchResponseKind::NotFound(id) => {
                return Err(use_case_err(
                    format!("Invalid account id: {}", id.unwrap()),
                    Some(true),
                    None,
                ))
            }
            FetchResponseKind::Found(res) => res,
        },
    };

    // ? ----------------------------------------------------------------------
    // ? Update role and persist
    // ? ----------------------------------------------------------------------

    role.name = name;
    role.description = description;

    role_updating_repo.update(role).await
}
