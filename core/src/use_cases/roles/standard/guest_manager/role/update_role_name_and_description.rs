use crate::domain::{
    actors::DefaultActor,
    dtos::{profile::Profile, role::Role},
    entities::{RoleFetching, RoleUpdating},
};
use clean_base::{
    entities::{FetchResponseKind, UpdatingResponseKind},
    utils::errors::{factories::use_case_err, MappedErrors},
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

    profile.get_update_ids_or_error(vec![
        DefaultActor::GuestManager.to_string()
    ])?;

    // ? ----------------------------------------------------------------------
    // ? Fetch desired role
    // ? ----------------------------------------------------------------------

    let mut role = match role_fetching_repo.get(role_id).await? {
        FetchResponseKind::NotFound(id) => {
            return use_case_err(format!("Invalid account id: {}", id.unwrap()))
                .as_error()
        }
        FetchResponseKind::Found(res) => res,
    };

    // ? ----------------------------------------------------------------------
    // ? Update role and persist
    // ? ----------------------------------------------------------------------

    role.name = name;
    role.description = description;

    role_updating_repo.update(role).await
}
